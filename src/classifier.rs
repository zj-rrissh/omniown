pub struct Classification {
    pub folder_type: String,
    pub category: String,
    pub domain: String,
    pub doc_type: String,
    pub privacy_score: f64,
    pub risk_level: String,
}

const PRIVACY_KEYWORDS: &[&str] = &[
    "身份证",
    "密码",
    "银行卡",
    "银行",
    "收入",
    "工资",
    "发票",
    "账单",
    "报销",
    "合同",
    "token",
    "secret",
    "api_key",
    "private_key",
    "日记",
    "心情",
    "情绪",
    "难过",
    "开心",
];

const FINANCE_KEYWORDS: &[&str] = &["发票", "账单", "报销", "银行", "银行卡", "收入", "工资"];

const IDENTITY_KEYWORDS: &[&str] = &[
    "身份证",
    "密码",
    "token",
    "secret",
    "api_key",
    "private_key",
];

const JOURNAL_KEYWORDS: &[&str] = &["日记", "心情", "情绪", "今天", "难过", "开心"];

const CODE_EXTENSIONS: &[&str] = &[
    "rs", "js", "ts", "jsx", "tsx", "py", "java", "go", "cpp", "c", "h", "hpp", "css", "sh", "sql",
];

const NOTE_EXTENSIONS: &[&str] = &["md", "markdown", "txt", "log"];

const DOC_EXTENSIONS: &[&str] = &["pdf", "doc", "docx", "html", "htm"];

const DATA_EXTENSIONS: &[&str] = &["json", "toml", "yaml", "yml", "csv"];

pub fn classify_document(filename: &str, content: &str) -> Classification {
    let combined = format!("{} {}", filename.to_lowercase(), content.to_lowercase());

    let is_private = PRIVACY_KEYWORDS.iter().any(|kw| combined.contains(kw));

    if is_private {
        let category = if FINANCE_KEYWORDS.iter().any(|kw| combined.contains(kw)) {
            "finance"
        } else if IDENTITY_KEYWORDS.iter().any(|kw| combined.contains(kw)) {
            "identity"
        } else if JOURNAL_KEYWORDS.iter().any(|kw| combined.contains(kw)) {
            "journal"
        } else {
            "misc"
        };

        let domain = match category {
            "finance" => "finance",
            "journal" | "identity" => "personal",
            _ => "unknown",
        };

        let risk_level = match category {
            "identity" => "high",
            "finance" | "journal" => "medium",
            _ => "medium",
        };

        return Classification {
            folder_type: "private".into(),
            category: category.into(),
            domain: domain.into(),
            doc_type: doc_type_from_filename(filename),
            privacy_score: 0.9,
            risk_level: risk_level.into(),
        };
    }

    let ext = filename
        .rsplit('.')
        .next()
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let category = if CODE_EXTENSIONS.contains(&ext.as_str()) {
        "code"
    } else if NOTE_EXTENSIONS.contains(&ext.as_str()) {
        "notes"
    } else if DOC_EXTENSIONS.contains(&ext.as_str()) {
        "docs"
    } else if DATA_EXTENSIONS.contains(&ext.as_str()) {
        "data"
    } else {
        "misc"
    };

    let domain = if category == "code" { "dev" } else { "unknown" };

    Classification {
        folder_type: "public".into(),
        category: category.into(),
        domain: domain.into(),
        doc_type: doc_type_from_filename(filename),
        privacy_score: 0.1,
        risk_level: "low".into(),
    }
}

fn doc_type_from_filename(filename: &str) -> String {
    let ext = filename
        .rsplit('.')
        .next()
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "md" => "markdown".into(),
        "markdown" => "markdown".into(),
        "txt" => "text".into(),
        "html" | "htm" => "html".into(),
        "json" | "toml" | "yaml" | "yml" => "config".into(),
        "csv" => "table".into(),
        "log" => "log".into(),
        "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "java" | "go" | "cpp" | "c" | "h" | "hpp"
        | "css" | "sh" | "sql" => "code".into(),
        "pdf" => "pdf".into(),
        "doc" | "docx" => "word".into(),
        _ => "unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_markdown_is_notes() {
        let c = classify_document("rust学习.md", "# Rust 笔记\n所有权和借用");
        assert_eq!(c.folder_type, "public");
        assert_eq!(c.category, "notes");
        assert_eq!(c.doc_type, "markdown");
        assert_eq!(c.privacy_score, 0.1);
        assert_eq!(c.risk_level, "low");
    }

    #[test]
    fn code_file_is_public_code() {
        let c = classify_document("main.rs", "fn main() {}");
        assert_eq!(c.folder_type, "public");
        assert_eq!(c.category, "code");
        assert_eq!(c.doc_type, "code");
        assert_eq!(c.domain, "dev");
    }

    #[test]
    fn finance_keyword_triggers_private() {
        let c = classify_document("报销账单.md", "本月发票和报销明细");
        assert_eq!(c.folder_type, "private");
        assert_eq!(c.category, "finance");
        assert_eq!(c.risk_level, "medium");
        assert_eq!(c.privacy_score, 0.9);
    }

    #[test]
    fn identity_keyword_triggers_private_high() {
        let c = classify_document("config.txt", "api_key=abc123 secret=xyz");
        assert_eq!(c.folder_type, "private");
        assert_eq!(c.category, "identity");
        assert_eq!(c.risk_level, "high");
    }

    #[test]
    fn journal_keyword_triggers_private_journal() {
        let c = classify_document("今日日记.md", "今天心情很好，很开心");
        assert_eq!(c.folder_type, "private");
        assert_eq!(c.category, "journal");
        assert_eq!(c.risk_level, "medium");
    }

    #[test]
    fn misc_private_fallback() {
        let c = classify_document("合同.txt", "这是一份合同文档");
        assert_eq!(c.folder_type, "private");
        assert_eq!(c.category, "misc");
    }

    #[test]
    fn unknown_extension_is_misc() {
        let c = classify_document("data.xyz", "some content");
        assert_eq!(c.folder_type, "public");
        assert_eq!(c.category, "misc");
        assert_eq!(c.doc_type, "unknown");
    }

    #[test]
    fn pdf_is_docs() {
        let c = classify_document("report.pdf", "dummy content");
        assert_eq!(c.folder_type, "public");
        assert_eq!(c.category, "docs");
        assert_eq!(c.doc_type, "pdf");
    }

    #[test]
    fn html_is_docs() {
        let c = classify_document("page.html", "Hello Rust");
        assert_eq!(c.folder_type, "public");
        assert_eq!(c.category, "docs");
        assert_eq!(c.doc_type, "html");
    }

    #[test]
    fn json_is_data() {
        let c = classify_document("data.json", r#"{"hello":"world"}"#);
        assert_eq!(c.folder_type, "public");
        assert_eq!(c.category, "data");
        assert_eq!(c.doc_type, "config");
    }
}
