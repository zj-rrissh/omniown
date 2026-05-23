use anyhow::{Context, bail};
use std::fs;
use std::path::Path;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "html", "htm", "rs", "js", "ts", "jsx", "tsx", "py", "java", "go",
    "cpp", "c", "h", "hpp", "css", "sh", "sql", "json", "toml", "yaml", "yml", "csv", "log",
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

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read UTF-8 text from {}", path.display()))?;

    let (format, text) = match file_ext.as_str() {
        "md" | "markdown" => ("markdown", extract_markdown_text(&raw)),
        "html" | "htm" => ("html", extract_html_text(&raw)),
        "json" => ("json", raw),
        "toml" => ("toml", raw),
        "yaml" | "yml" => ("yaml", raw),
        "csv" => ("csv", raw),
        "log" => ("log", raw),
        "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "java" | "go" | "cpp" | "c" | "h" | "hpp"
        | "css" | "sh" | "sql" => ("code", raw),
        _ => ("text", raw),
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
            cursor = raw.len();
            break;
        };
        let content_start = start + relative_open_end + 1;

        let Some(relative_close_start) = find_bytes(&lower[content_start..], close.as_bytes())
        else {
            cursor = raw.len();
            break;
        };
        let close_start = content_start + relative_close_start;

        let Some(relative_close_end) = lower[close_start..].iter().position(|byte| *byte == b'>')
        else {
            cursor = raw.len();
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
        assert!(!is_supported_path(Path::new("doc.pdf")));
        assert!(!is_supported_path(Path::new("image.png")));
        assert!(!is_supported_path(Path::new("archive.zip")));
        assert!(!is_supported_path(Path::new("binary.bin")));
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
    fn remove_tag_blocks_missing_close_drops_rest() {
        // Current limitation: content after an unclosed block is dropped.
        let input = "before<script>no close after";
        let result = remove_tag_blocks(input, "script");
        assert_eq!(result, "before");
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
        let path = temp_file("doc.pdf", "fake pdf content");
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
}
