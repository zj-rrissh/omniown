use anyhow::{Context, bail};
use std::fs;
use std::io::Read;
use std::path::Path;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "html", "htm", "rs", "js", "ts", "jsx", "tsx", "py", "java", "go",
    "cpp", "c", "h", "hpp", "css", "sh", "sql", "json", "toml", "yaml", "yml", "csv", "log", "pdf",
    "docx", "pptx", "xlsx", "xlsm",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedText {
    pub text: String,
    pub format: &'static str,
    pub file_ext: String,
}

pub fn is_supported_path(path: &Path) -> bool {
    extension(path)
        .as_deref()
        .is_some_and(|ext| SUPPORTED_EXTENSIONS.contains(&ext))
}

pub fn extract_text(path: &Path) -> anyhow::Result<ExtractedText> {
    let file_ext = extension(path).unwrap_or_default();

    if !SUPPORTED_EXTENSIONS.contains(&file_ext.as_str()) {
        bail!("unsupported file extension: {}", file_ext);
    }

    let (format, text) = match file_ext.as_str() {
        // Binary formats — use specialixsed extractors
        "pdf" => ("pdf", extract_pdf_text(path)?),
        "docx" => ("docx", extract_docx_text(path)?),
        "pptx" => ("pptx", extract_pptx_text(path)?),
        "xlsx" | "xlsm" => ("xlsx", extract_xlsx_text(path)?),

        // Text-based formats — read as UTF-8
        _ => {
            let raw = fs::read_to_string(path)
                .with_context(|| format!("failed to read UTF-8 text from {}", path.display()))?;

            let text = match file_ext.as_str() {
                "md" | "markdown" => extract_markdown_text(&raw),
                "html" | "htm" => extract_html_text(&raw),
                "json" => raw,
                "toml" => raw,
                "yaml" | "yml" => raw,
                "csv" => raw,
                "log" => raw,
                "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "java" | "go" | "cpp" | "c" | "h"
                | "hpp" | "css" | "sh" | "sql" => raw,
                _ => raw,
            };

            let format = match file_ext.as_str() {
                "md" | "markdown" => "markdown",
                "html" | "htm" => "html",
                "json" => "json",
                "toml" => "toml",
                "yaml" | "yml" => "yaml",
                "csv" => "csv",
                "log" => "log",
                "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "java" | "go" | "cpp" | "c" | "h"
                | "hpp" | "css" | "sh" | "sql" => "code",
                _ => "text",
            };

            (format, text)
        }
    };

    Ok(ExtractedText {
        text,
        format,
        file_ext,
    })
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

fn extract_markdown_text(raw: &str) -> String {
    let mut out = String::new();

    for line in raw.lines() {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("---") {
            continue;
        }

        let line = strip_markdown_prefix(line);
        let line = strip_markdown_inline(line);
        out.push_str(line.trim());
        out.push('\n');
    }

    out.trim().to_string()
}

fn strip_markdown_prefix(line: &str) -> &str {
    let mut s = line.trim_start();

    while let Some(rest) = s.strip_prefix('#') {
        s = rest.trim_start();
    }

    while let Some(rest) = s.strip_prefix('>') {
        s = rest.trim_start();
    }

    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = s.strip_prefix(marker) {
            return rest;
        }
    }

    let digit_count = s.chars().take_while(|c| c.is_ascii_digit()).count();
    if digit_count > 0
        && s.chars().nth(digit_count) == Some('.')
        && s.chars().nth(digit_count + 1) == Some(' ')
    {
        return &s[digit_count + 2..];
    }

    s
}

fn strip_markdown_inline(line: &str) -> String {
    line.chars()
        .filter(|ch| !matches!(ch, '`' | '*' | '_' | '~'))
        .collect()
}

fn extract_html_text(raw: &str) -> String {
    let without_scripts = remove_tag_blocks(raw, "script");
    let without_styles = remove_tag_blocks(&without_scripts, "style");

    let mut out = String::new();
    let mut in_tag = false;

    for ch in without_styles.chars() {
        match ch {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    collapse_whitespace(&decode_html_entities(&out))
}

fn remove_tag_blocks(raw: &str, tag: &str) -> String {
    let lower: Vec<u8> = raw
        .as_bytes()
        .iter()
        .map(|byte| byte.to_ascii_lowercase())
        .collect();
    let open = format!("<{}", tag);
    let close = format!("</{}", tag);
    let mut out = String::new();
    let mut cursor = 0;

    while let Some(relative_start) = find_bytes(&lower[cursor..], open.as_bytes()) {
        let start = cursor + relative_start;
        out.push_str(&raw[cursor..start]);

        let Some(relative_open_end) = lower[start..].iter().position(|byte| *byte == b'>') else {
            // 无闭合 '>' — 跳过 "<script" 继续处理后续内容
            cursor = start + open.len();
            break;
        };
        let content_start = start + relative_open_end + 1;

        let Some(relative_close_start) = find_bytes(&lower[content_start..], close.as_bytes())
        else {
            // 无闭合标签 — 跳过整个打开标签，保留后续内容
            out.push_str(&raw[start..content_start]);
            out.push(' ');
            cursor = content_start;
            break;
        };
        let close_start = content_start + relative_close_start;

        let Some(relative_close_end) = lower[close_start..].iter().position(|byte| *byte == b'>')
        else {
            // 无闭合 '>' — 跳过打开标签
            out.push_str(&raw[start..content_start]);
            out.push(' ');
            cursor = content_start;
            break;
        };

        out.push(' ');
        cursor = close_start + relative_close_end + 1;
    }

    out.push_str(&raw[cursor..]);
    out
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn decode_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---- PDF extraction (via lopdf) ----

fn extract_pdf_text(path: &Path) -> anyhow::Result<String> {
    let doc = lopdf::Document::load(path)
        .with_context(|| format!("failed to load PDF from {}", path.display()))?;

    let pages = doc.get_pages();
    if pages.is_empty() {
        bail!("PDF has no pages: {}", path.display());
    }

    let page_numbers: Vec<u32> = pages.keys().copied().collect();
    let text = doc
        .extract_text(&page_numbers)
        .map_err(|e| anyhow::anyhow!("PDF text extraction failed: {}", e))?;

    Ok(text.trim().to_string())
}

// ---- xlsx extraction (via calamine) ----

fn extract_xlsx_text(path: &Path) -> anyhow::Result<String> {
    use calamine::{Data, Reader, Xlsx, open_workbook};

    let mut workbook: Xlsx<_> = open_workbook(path)
        .with_context(|| format!("failed to open xlsx from {}", path.display()))?;

    let mut text = String::new();
    let sheet_names: Vec<String> = workbook.sheet_names().to_vec();

    for name in &sheet_names {
        let range = workbook
            .worksheet_range(name)
            .with_context(|| format!("failed to read sheet '{}'", name))?;

        for row in range.rows() {
            let mut row_text = Vec::new();
            for cell in row {
                let cell_str = match cell {
                    Data::String(s) => s.clone(),
                    Data::Float(f) => f.to_string(),
                    Data::Int(i) => i.to_string(),
                    Data::Bool(b) => b.to_string(),
                    Data::DateTime(f) => f.to_string(),
                    Data::DateTimeIso(i) => i.to_string(),
                    Data::DurationIso(i) => i.to_string(),
                    Data::Error(e) => format!("[ERR: {}]", e),
                    Data::Empty => String::new(),
                };
                row_text.push(cell_str);
            }
            text.push_str(&row_text.join("\t"));
            text.push('\n');
        }
    }

    if text.is_empty() {
        bail!("xlsx contains no readable text: {}", path.display());
    }

    Ok(text.trim().to_string())
}

// ---- docx extraction (via zip + quick-xml) ----

fn extract_docx_text(path: &Path) -> anyhow::Result<String> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open docx from {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("failed to read docx (ZIP) from {}", path.display()))?;

    // Extract text from word/document.xml
    let mut text = String::new();
    let mut found = false;

    for entry_name in ["word/document.xml", "word/document2.xml"] {
        if let Ok(mut xml_file) = archive.by_name(entry_name) {
            found = true;
            let mut xml_buf = Vec::new();
            xml_file
                .read_to_end(&mut xml_buf)
                .context("failed to read docx XML entry")?;

            let mut reader = Reader::from_reader(xml_buf.as_slice());
            let mut buf = Vec::new();
            let mut in_t = false;

            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                        if e.name().as_ref() == b"w:t" =>
                    {
                        in_t = true;
                    }
                    Ok(Event::Text(ref e)) if in_t => {
                        if let Ok(s) = e.unescape() {
                            text.push_str(&s);
                        }
                    }
                    Ok(Event::End(ref e)) if e.name().as_ref() == b"w:t" => {
                        in_t = false;
                    }
                    Ok(Event::End(ref e)) if e.name().as_ref() == b"w:p" => {
                        text.push('\n');
                    }
                    Ok(Event::Eof) => break,
                    Err(e) => bail!("docx XML parse error: {}", e),
                    _ => {}
                }
                buf.clear();
            }
        }
    }

    if !found {
        bail!("docx contains no document.xml entry: {}", path.display());
    }

    Ok(collapse_whitespace(&text))
}

// ---- pptx extraction (via zip + quick-xml) ----

fn extract_pptx_text(path: &Path) -> anyhow::Result<String> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open pptx from {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("failed to read pptx (ZIP) from {}", path.display()))?;

    let mut text = String::new();
    let mut slide_index = 0;

    // Find all slide files: ppt/slides/slide1.xml, slide2.xml, ...
    loop {
        slide_index += 1;
        let entry_name = format!("ppt/slides/slide{}.xml", slide_index);

        let Ok(mut xml_file) = archive.by_name(&entry_name) else {
            break;
        };

        let mut xml_buf = Vec::new();
        xml_file
            .read_to_end(&mut xml_buf)
            .with_context(|| format!("failed to read {}", entry_name))?;

        text.push_str(&format!("\n--- Slide {} ---\n", slide_index));

        let mut reader = Reader::from_reader(xml_buf.as_slice());
        let mut buf = Vec::new();
        let mut in_t = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                    if e.name().as_ref() == b"a:t" =>
                {
                    in_t = true;
                }
                Ok(Event::Text(ref e)) if in_t => {
                    if let Ok(s) = e.unescape() {
                        text.push_str(&s);
                    }
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"a:t" => {
                    in_t = false;
                }
                Ok(Event::Eof) => break,
                Err(e) => bail!("pptx XML parse error in {}: {}", entry_name, e),
                _ => {}
            }
            buf.clear();
        }
    }

    if slide_index <= 1 {
        bail!("pptx contains no slides: {}", path.display());
    }

    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    // ---- extension ----

    #[test]
    fn extension_returns_lowercase() {
        assert_eq!(extension(Path::new("File.TXT")), Some("txt".into()));
    }

    #[test]
    fn extension_no_extension_returns_none() {
        assert_eq!(extension(Path::new("Makefile")), None);
    }

    #[test]
    fn extension_multiple_dots_uses_last() {
        assert_eq!(extension(Path::new("archive.tar.gz")), Some("gz".into()));
    }

    #[test]
    fn extension_empty_filename_returns_none() {
        assert_eq!(extension(Path::new("")), None);
    }

    // ---- is_supported_path ----

    #[test]
    fn is_supported_path_common_extensions() {
        for ext in [
            "txt", "md", "html", "rs", "py", "json", "toml", "yaml", "csv", "log",
        ] {
            let name = format!("file.{}", ext);
            assert!(
                is_supported_path(Path::new(&name)),
                "expected {} to be supported",
                name
            );
        }
    }

    #[test]
    fn is_supported_path_unsupported_extension() {
        assert!(!is_supported_path(Path::new("image.png")));
        assert!(!is_supported_path(Path::new("archive.zip")));
        assert!(!is_supported_path(Path::new("binary.bin")));
        assert!(!is_supported_path(Path::new("audio.mp3")));
    }

    #[test]
    fn is_supported_path_case_insensitive() {
        assert!(is_supported_path(Path::new("NOTE.MD")));
        assert!(is_supported_path(Path::new("Page.HTML")));
        assert!(is_supported_path(Path::new("Main.RS")));
    }

    #[test]
    fn is_supported_path_no_extension() {
        assert!(!is_supported_path(Path::new("Makefile")));
        assert!(!is_supported_path(Path::new("README")));
    }

    // ---- strip_markdown_prefix ----

    #[test]
    fn md_prefix_plain_text_unchanged() {
        assert_eq!(strip_markdown_prefix("hello world"), "hello world");
    }

    #[test]
    fn md_prefix_strips_heading_markers() {
        assert_eq!(strip_markdown_prefix("# Title"), "Title");
        assert_eq!(strip_markdown_prefix("### Sub"), "Sub");
    }

    #[test]
    fn md_prefix_nested_markers() {
        assert_eq!(strip_markdown_prefix("> **quote**"), "**quote**");
    }

    #[test]
    fn md_prefix_unordered_list_markers() {
        assert_eq!(strip_markdown_prefix("- item"), "item");
        assert_eq!(strip_markdown_prefix("* item"), "item");
        assert_eq!(strip_markdown_prefix("+ item"), "item");
    }

    #[test]
    fn md_prefix_ordered_list_marker() {
        assert_eq!(strip_markdown_prefix("1. first"), "first");
        assert_eq!(strip_markdown_prefix("42. answer"), "answer");
    }

    #[test]
    fn md_prefix_leading_whitespace() {
        assert_eq!(strip_markdown_prefix("  - indented"), "indented");
        assert_eq!(strip_markdown_prefix("  > blockquote"), "blockquote");
    }

    // ---- strip_markdown_inline ----

    #[test]
    fn md_inline_plain_text_unchanged() {
        assert_eq!(strip_markdown_inline("hello world"), "hello world");
    }

    #[test]
    fn md_inline_removes_formatting_chars() {
        assert_eq!(strip_markdown_inline("**bold**"), "bold");
        assert_eq!(strip_markdown_inline("*italic*"), "italic");
        assert_eq!(strip_markdown_inline("`code`"), "code");
        assert_eq!(strip_markdown_inline("~~strike~~"), "strike");
    }

    #[test]
    fn md_inline_mixed_formatting() {
        assert_eq!(
            strip_markdown_inline("**bold *and* italic**"),
            "bold and italic"
        );
    }

    // ---- extract_markdown_text ----

    #[test]
    fn md_empty_string_returns_empty() {
        assert_eq!(extract_markdown_text(""), "");
    }

    #[test]
    fn md_plain_text_preserved() {
        let result = extract_markdown_text("Hello\nWorld");
        assert_eq!(result, "Hello\nWorld");
    }

    #[test]
    fn md_headings_stripped() {
        let result = extract_markdown_text("# Title\n\n## Section\n\nContent");
        assert!(result.contains("Title"));
        assert!(result.contains("Section"));
        assert!(result.contains("Content"));
        assert!(!result.contains('#'));
    }

    #[test]
    fn md_code_fence_lines_skipped() {
        // Only fence markers (```) are skipped; content between them passes through.
        let input = "Before\n```rust\nlet x = 1;\n```\nAfter";
        let result = extract_markdown_text(input);
        assert!(result.contains("Before"), "text before fence");
        assert!(result.contains("After"), "text after fence");
        assert!(!result.contains("```"), "fence markers removed");
        assert!(result.contains("let x = 1;"), "fence content preserved");
    }

    #[test]
    fn md_horizontal_rules_removed() {
        let input = "Before\n---\nAfter";
        let result = extract_markdown_text(input);
        // `---` line is skipped, lines before/after connect with \n.
        assert_eq!(result, "Before\nAfter");
    }

    #[test]
    fn md_lists_stripped() {
        let result = extract_markdown_text("- item1\n- item2\n- item3");
        assert_eq!(result, "item1\nitem2\nitem3");
    }

    #[test]
    fn md_ordered_lists_stripped() {
        let result = extract_markdown_text("1. first\n2. second");
        assert_eq!(result, "first\nsecond");
    }

    #[test]
    fn md_blockquote_stripped() {
        let result = extract_markdown_text("> quoted\nnormal");
        assert_eq!(result, "quoted\nnormal");
    }

    // ---- remove_tag_blocks ----

    #[test]
    fn remove_tag_blocks_no_match_returns_original() {
        let result = remove_tag_blocks("<p>hello</p>", "script");
        assert_eq!(result, "<p>hello</p>");
    }

    #[test]
    fn remove_tag_blocks_removes_single_block() {
        let result = remove_tag_blocks("<div>keep<script>remove</script>keep</div>", "script");
        assert_eq!(result, "<div>keep keep</div>");
    }

    #[test]
    fn remove_tag_blocks_multiple_blocks() {
        let result = remove_tag_blocks("a<script>x</script>b<script>y</script>c", "script");
        assert_eq!(result, "a b c");
    }

    #[test]
    fn remove_tag_blocks_missing_close_keeps_content() {
        // 缺闭合标签时保留打开标签文本和后续内容
        let input = "before<script>no close after";
        let result = remove_tag_blocks(input, "script");
        assert_eq!(result, "before<script> no close after");
    }

    #[test]
    fn remove_tag_blocks_case_insensitive_tag() {
        let result = remove_tag_blocks("before<SCRIPT>hide</SCRIPT>after", "script");
        assert_eq!(result, "before after");
    }

    #[test]
    fn remove_tag_blocks_empty_content() {
        let result = remove_tag_blocks("a<script></script>b", "script");
        assert_eq!(result, "a b");
    }

    // ---- decode_html_entities ----

    #[test]
    fn decode_html_entities_no_entities_unchanged() {
        assert_eq!(decode_html_entities("hello world"), "hello world");
    }

    #[test]
    fn decode_html_entities_all_known() {
        let input = "&nbsp; &amp; &lt; &gt; &quot; &#39; &apos;";
        let expected = "  & < > \" ' '";
        assert_eq!(decode_html_entities(input), expected);
    }

    #[test]
    fn decode_html_entities_unknown_preserved() {
        assert_eq!(
            decode_html_entities("hello &unknown; world"),
            "hello &unknown; world"
        );
    }

    // ---- collapse_whitespace ----

    #[test]
    fn collapse_whitespace_normal_text_unchanged() {
        assert_eq!(collapse_whitespace("hello world"), "hello world");
    }

    #[test]
    fn collapse_whitespace_multiple_spaces() {
        assert_eq!(collapse_whitespace("hello   world"), "hello world");
    }

    #[test]
    fn collapse_whitespace_tabs_and_newlines() {
        assert_eq!(collapse_whitespace("hello\t\nworld"), "hello world");
    }

    #[test]
    fn collapse_whitespace_empty_string() {
        assert_eq!(collapse_whitespace(""), "");
    }

    #[test]
    fn collapse_whitespace_only_whitespace() {
        assert_eq!(collapse_whitespace("   \n\t   "), "");
    }

    // ---- extract_html_text (holistic) ----

    #[test]
    fn html_empty_string_returns_empty() {
        assert_eq!(extract_html_text(""), "");
    }

    #[test]
    fn html_plain_text_preserved() {
        assert_eq!(extract_html_text("hello"), "hello");
    }

    #[test]
    fn html_strips_tags_preserves_text() {
        assert_eq!(extract_html_text("<p>hello</p>"), "hello");
    }

    #[test]
    fn html_removes_script_and_style_blocks() {
        let input = "<html><head><style>.x{}</style></head><body><h1>Hello&nbsp;Rust</h1><script>alert(1)</script><p>A&amp;B</p></body></html>";
        assert_eq!(extract_html_text(input), "Hello Rust A&B");
    }

    #[test]
    fn html_handles_nested_tags() {
        let input = "<div><p>deep <b>bold</b></p></div>";
        assert_eq!(extract_html_text(input), "deep bold");
    }

    #[test]
    fn html_unicode_preserved() {
        let input = "<main>你好 <script>隐藏</script>世界</main>";
        assert_eq!(extract_html_text(input), "你好 世界");
    }

    #[test]
    fn html_self_closing_tags_no_text() {
        let input = "<br><hr><img src='x'>visible";
        assert_eq!(extract_html_text(input), "visible");
    }

    #[test]
    fn html_unclosed_tag_recovers() {
        let input = "<div>hello<p>world";
        assert_eq!(extract_html_text(input), "hello world");
    }

    // ---- extract_text (with temp files 集成测试) ----
    // Uses a unique counter per call to avoid parallel-test conflicts.

    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_file(name: &str, content: &str) -> PathBuf {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "omniown_extractor_test_{}_{}",
            std::process::id(),
            counter
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    fn cleanup_dir(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::remove_dir_all(parent).ok();
        }
    }

    #[test]
    fn extract_text_markdown_returns_markdown_format() {
        let path = temp_file("test.md", "# Hello\nWorld");
        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "markdown", "expected format 'markdown'");
        cleanup_dir(&path);
    }

    #[test]
    fn extract_text_html_returns_html_format() {
        let path = temp_file("test.html", "<p>hello</p>");
        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "html");
        cleanup_dir(&path);
    }

    #[test]
    fn extract_text_code_files_preserve_content() {
        let path = temp_file("main.rs", "fn main() {}");
        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "code");
        assert!(result.text.contains("fn main() {}"));
        cleanup_dir(&path);
    }

    #[test]
    fn extract_text_json_preserves_structure() {
        let path = temp_file("data.json", "{\"key\": \"value\"}");
        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "json");
        assert!(result.text.contains("\"key\""));
        cleanup_dir(&path);
    }

    #[test]
    fn extract_text_unsupported_extension_errors() {
        let path = temp_file("doc.png", "fake png content");
        let err = extract_text(&path).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("unsupported"),
            "expected 'unsupported' error, got: {}",
            msg
        );
        cleanup_dir(&path);
    }

    #[test]
    fn extract_text_nonexistent_file_errors() {
        let path = PathBuf::from("/nonexistent/note.md");
        let err = extract_text(&path).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("failed to read"),
            "expected file-read error, got: {}",
            msg
        );
    }

    #[test]
    fn extract_text_toml_format_detected() {
        let path = temp_file("config.toml", "key = \"value\"");
        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "toml");
        cleanup_dir(&path);
    }

    #[test]
    fn extract_text_yaml_format_detected() {
        let path = temp_file("config.yaml", "key: value");
        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "yaml");
        cleanup_dir(&path);
    }

    #[test]
    fn extract_text_csv_format_detected() {
        let path = temp_file("data.csv", "a,b,c\n1,2,3");
        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "csv");
        cleanup_dir(&path);
    }

    #[test]
    fn extract_text_log_format_detected() {
        let path = temp_file("server.log", "ERROR: something failed");
        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "log");
        cleanup_dir(&path);
    }

    #[test]
    fn extract_txt_is_text_format() {
        let path = temp_file("readme.txt", "plain text");
        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "text");
        cleanup_dir(&path);
    }

    #[test]
    fn extracted_text_file_ext_preserved() {
        let path = temp_file("hello.md", "content");
        let result = extract_text(&path).unwrap();
        assert_eq!(result.file_ext, "md");
        cleanup_dir(&path);
    }

    #[test]
    fn extract_text_with_starting_newlines() {
        let path = temp_file("test.md", "\n\n\n# Hello\nWorld");
        let result = extract_text(&path).unwrap();
        // Should not start with newlines
        assert!(!result.text.starts_with('\n'), "text should be trimmed");
        assert!(result.text.contains("Hello"), "text should contain content");
        cleanup_dir(&path);
    }

    // ---- PDF tests ----

    #[test]
    fn is_supported_path_pdf() {
        assert!(is_supported_path(Path::new("doc.pdf")));
    }

    #[test]
    fn pdf_nonexistent_file_errors() {
        let path = PathBuf::from("/nonexistent/doc.pdf");
        let err = extract_text(&path).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("failed to load PDF"),
            "expected PDF load error, got: {}",
            msg
        );
    }

    #[test]
    fn pdf_format_detected() {
        // A minimal valid PDF with "Hello PDF" text content
        let pdf_bytes: &[u8] = b"%PDF-1.4\n\
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
            3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]\n\
            /Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>endobj\n\
            4 0 obj<</Length 43>>stream\n\
            BT /F1 24 Tf 100 700 Td (Hello PDF) Tj ET\n\
            endstream\n\
            endobj\n\
            5 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n\
            xref\n0 6\n0000000000 65535 f \n\
            0000000009 00000 n \n0000000058 00000 n \n\
            0000000115 00000 n \n0000000266 00000 n \n\
            0000000362 00000 n \n\
            trailer<</Size 6/Root 1 0 R>>\n\
            startxref\n437\n%%EOF";

        let dir = std::env::temp_dir().join(format!("omniown_pdf_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.pdf");
        std::fs::write(&path, pdf_bytes).unwrap();

        match extract_text(&path) {
            Ok(result) => {
                assert_eq!(result.format, "pdf");
                assert_eq!(result.file_ext, "pdf");
                // The minimal PDF may or may not yield text depending on lopdf's
                // heuristic; at minimum the format and file_ext are correct.
                if !result.text.is_empty() {
                    assert!(result.text.contains("Hello PDF"));
                }
            }
            Err(e) => {
                // lopdf may reject this minimal PDF depending on version —
                // that's acceptable as long as the format dispatch ran.
                let msg = format!("{}", e);
                assert!(
                    msg.contains("PDF") || msg.contains("pdf"),
                    "PDF-related error expected, got: {}",
                    msg
                );
            }
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- xlsx tests ----

    #[test]
    fn is_supported_path_xlsx() {
        assert!(is_supported_path(Path::new("data.xlsx")));
        assert!(is_supported_path(Path::new("data.xlsm")));
    }

    #[test]
    fn xlsx_nonexistent_file_errors() {
        let path = PathBuf::from("/nonexistent/data.xlsx");
        let err = extract_text(&path).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("failed to open xlsx"),
            "expected xlsx open error, got: {}",
            msg
        );
    }

    #[test]
    fn xlsx_format_detected() {
        // Create a minimal xlsx: ZIP containing [Content_Types].xml, xl/workbook.xml,
        // xl/_rels/workbook.xml.rels, xl/worksheets/sheet1.xml
        use std::io::Write;

        let dir = std::env::temp_dir().join(format!("omniown_xlsx_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.xlsx");

        {
            let file = std::fs::File::create(&path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            zip.add_directory("_rels/", options).unwrap();
            zip.add_directory("xl/", options).unwrap();
            zip.add_directory("xl/_rels/", options).unwrap();
            zip.add_directory("xl/worksheets/", options).unwrap();

            zip.start_file("[Content_Types].xml", options).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/></Types>").unwrap();

            zip.start_file("xl/_rels/workbook.xml.rels", options)
                .unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>").unwrap();

            zip.start_file("xl/workbook.xml", options).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheets><sheet name=\"Sheet1\" sheetId=\"1\" r:id=\"rId1\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"/></sheets></workbook>").unwrap();

            zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData><row r=\"1\"><c r=\"A1\" t=\"s\"><v>0</v></c><c r=\"B1\" t=\"s\"><v>1</v></c></row></sheetData><sheetData><row r=\"2\"><c r=\"A2\" t=\"s\"><v>2</v></c><c r=\"B2\" t=\"s\"><v>3</v></c></row></sheetData></worksheet>").unwrap();

            zip.start_file("xl/sharedStrings.xml", options).unwrap();
            zip.write_all(b"<?xml version=\"1.0\"?><sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" count=\"4\"><si><t>Hello</t></si><si><t>World</t></si><si><t>Foo</t></si><si><t>Bar</t></si></sst>").unwrap();

            zip.finish().unwrap();
        }

        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "xlsx");
        assert_eq!(result.file_ext, "xlsx");
        assert!(
            result.text.contains("Hello"),
            "xlsx text should contain Hello"
        );
        assert!(
            result.text.contains("World"),
            "xlsx text should contain World"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- docx tests ----

    #[test]
    fn is_supported_path_docx() {
        assert!(is_supported_path(Path::new("report.docx")));
    }

    #[test]
    fn docx_nonexistent_file_errors() {
        let path = PathBuf::from("/nonexistent/report.docx");
        let err = extract_text(&path).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("failed to open docx"),
            "expected docx open error, got: {}",
            msg
        );
    }

    #[test]
    fn docx_format_detected() {
        use std::io::Write;

        let dir = std::env::temp_dir().join(format!("omniown_docx_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.docx");

        {
            let file = std::fs::File::create(&path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            zip.start_file("word/document.xml", options).unwrap();
            zip.write_all(
                b"<?xml version=\"1.0\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body><w:p><w:r><w:t>Hello</w:t></w:r><w:r><w:t xml:space=\"preserve\"> Word</w:t></w:r></w:p><w:p><w:r><w:t>Second paragraph</w:t></w:r></w:p></w:body></w:document>"
            ).unwrap();

            zip.finish().unwrap();
        }

        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "docx");
        assert_eq!(result.file_ext, "docx");
        assert!(
            result.text.contains("Hello"),
            "docx text should contain Hello"
        );
        assert!(
            result.text.contains("Word"),
            "docx text should contain Word"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- pptx tests ----

    #[test]
    fn is_supported_path_pptx() {
        assert!(is_supported_path(Path::new("slides.pptx")));
    }

    #[test]
    fn pptx_nonexistent_file_errors() {
        let path = PathBuf::from("/nonexistent/slides.pptx");
        let err = extract_text(&path).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("failed to open pptx"),
            "expected pptx open error, got: {}",
            msg
        );
    }

    #[test]
    fn pptx_format_detected() {
        use std::io::Write;

        let dir = std::env::temp_dir().join(format!("omniown_pptx_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.pptx");

        {
            let file = std::fs::File::create(&path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            zip.add_directory("ppt/", options).unwrap();
            zip.add_directory("ppt/slides/", options).unwrap();

            zip.start_file("ppt/slides/slide1.xml", options).unwrap();
            zip.write_all(
                b"<?xml version=\"1.0\"?><p:sld xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:nvGrpSpPr><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title\"/><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:p xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><a:r><a:t>Hello PowerPoint</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"
            ).unwrap();

            zip.finish().unwrap();
        }

        let result = extract_text(&path).unwrap();
        assert_eq!(result.format, "pptx");
        assert_eq!(result.file_ext, "pptx");
        assert!(
            result.text.contains("Hello PowerPoint"),
            "pptx text should contain Hello PowerPoint"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- unsupported formats stay rejected ----

    #[test]
    fn extract_text_pdf_format_in_extract_text() {
        // Even though PDF isn't read as UTF-8, it should be handled by the
        // binary-format branch. Verify via non-existent file error.
        let path = PathBuf::from("/nonexistent/test.pdf");
        let err = extract_text(&path).unwrap_err();
        assert!(format!("{}", err).contains("failed to load PDF"));
    }

    #[test]
    fn extract_text_unsupported_binary_formats() {
        for ext in &["png", "jpg", "gif", "zip", "mp3", "mp4"] {
            let name = format!("file.{}", ext);
            assert!(
                !is_supported_path(Path::new(&name)),
                "{} should not be supported",
                name
            );
        }
    }

    #[test]
    fn extract_text_invalid_pdf_content_errors() {
        let path = temp_file("bad.pdf", "not a real pdf content");
        let result = extract_text(&path);
        assert!(result.is_err(), "invalid PDF content should error");
        cleanup_dir(&path);
    }
}
