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
    use std::path::PathBuf;

    #[test]
    fn supports_extensions_case_insensitively() {
        assert!(is_supported_path(&PathBuf::from("NOTE.MD")));
        assert!(is_supported_path(&PathBuf::from("page.HTML")));
        assert!(!is_supported_path(&PathBuf::from("archive.zip")));
    }

    #[test]
    fn markdown_text_is_lightly_normalized() {
        let text =
            extract_markdown_text("# Title\n\n- **Important** note\n```rust\nignored fence\n```");
        assert!(text.contains("Title"));
        assert!(text.contains("Important note"));
        assert!(!text.contains("```"));
    }

    #[test]
    fn html_text_strips_markup_and_script_content() {
        let text = extract_html_text(
            "<html><head><style>.x{}</style></head><body><h1>Hello&nbsp;Rust</h1><script>alert(1)</script><p>A&amp;B</p></body></html>",
        );
        assert_eq!(text, "Hello Rust A&B");
    }

    #[test]
    fn html_extraction_keeps_unicode_text_boundaries() {
        let text = extract_html_text("<main>你好 <script>隐藏</script>世界</main>");
        assert_eq!(text, "你好 世界");
    }
}
