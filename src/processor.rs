use crate::db::{self, NewDocument};

// ---- 从 classifier.rs 内联 ----

pub struct Classification {
    pub folder_type: String,
    pub category: String,
    pub domain: String,
    pub doc_type: String,
    pub privacy_score: f64,
    pub risk_level: String,
}

/// index_file_in_place 的返回值，指示文件是否在索引过程中被移动
pub struct IndexResult {
    /// 如果文件被移动到新路径，此字段为 Some(new_path)
    pub moved_to: Option<PathBuf>,
    /// 数据库记录是否发生了新增或更新；未变化的重复事件保持静默
    pub changed: bool,
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
const MAX_CLASSIFY_CHARS: usize = 64_000;

pub fn classify_document(filename: &str, content: &str) -> Classification {
    let content_prefix = if content.len() > MAX_CLASSIFY_CHARS {
        &content[..MAX_CLASSIFY_CHARS]
    } else {
        content
    };
    let combined = format!(
        "{} {}",
        filename.to_lowercase(),
        content_prefix.to_lowercase()
    );

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
use crate::extractor;
use crate::fs_layout::AppPaths;
use chrono::Local;
use std::fs;
use std::io;
use std::io::IsTerminal;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistingFileDecision {
    Overwrite,
    Cancel,
}

// ---- 从 storage.rs 内联 ----

pub fn build_stored_path(
    library_dir: &Path,
    filename: &str,
    _file_hash: &str,
    folder_type: &str,
) -> PathBuf {
    let safe_name = sanitize_filename(filename);
    library_dir.join(folder_type).join(safe_name)
}

#[allow(dead_code)]
pub fn is_old_library_filename(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() <= 20 {
        return false;
    }

    is_digit(bytes[0])
        && is_digit(bytes[1])
        && is_digit(bytes[2])
        && is_digit(bytes[3])
        && bytes[4] == b'-'
        && is_digit(bytes[5])
        && is_digit(bytes[6])
        && bytes[7] == b'-'
        && is_digit(bytes[8])
        && is_digit(bytes[9])
        && bytes[10] == b'_'
        && bytes[11..19].iter().all(|b| b.is_ascii_hexdigit())
        && bytes[19] == b'_'
}

#[allow(dead_code)]
fn is_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' => '_',
            other => other,
        })
        .collect();

    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn is_supported_file(path: &Path) -> bool {
    extractor::is_supported_path(path)
}

pub fn process_file(path: &Path, app_paths: &AppPaths) -> anyhow::Result<()> {
    process_file_with_conflict_decision(path, app_paths, None)
}

pub fn process_file_with_conflict_decision(
    path: &Path,
    app_paths: &AppPaths,
    conflict_decision: Option<ExistingFileDecision>,
) -> anyhow::Result<()> {
    if !is_supported_file(path) {
        println!("\u{23ed}\u{fe0f} 跳过不支持的文件类型: {:?}", path);
        return Ok(());
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let original_path = path.to_string_lossy().to_string();

    let extracted = match extractor::extract_text(path) {
        Ok(extracted) => extracted,
        Err(e) => {
            let reason = "extract_failed";
            log_failure(&app_paths.logs, &original_path, reason, &e.to_string());
            move_to_quarantine(path, &app_paths.quarantine, filename, reason).ok();
            return Ok(());
        }
    };

    let content = extracted.text;
    let file_size = fs::metadata(path).ok().map(|m| m.len() as i64);
    let file_hash = db::compute_hash(&content);
    let classification = classify_document(filename, &content);

    let stored_path = build_stored_path(
        &app_paths.library,
        filename,
        &file_hash,
        &classification.folder_type,
    );

    let stored_path_str = stored_path_for_db(&stored_path, app_paths)
        .to_string_lossy()
        .to_string();

    if let Some(parent) = stored_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        log_failure(&app_paths.logs, &original_path, "mkdir", &e.to_string());
        eprintln!("\u{26a0}\u{fe0f} 创建目标目录失败 [{}]: {}", filename, e);
        return Ok(());
    }

    // 文件已在 library 目标位置 → 跳过移动，直接索引
    let is_in_place = stored_path == path;

    let should_overwrite = if stored_path.exists() {
        // 源和目标相同 → 文件已在正确位置，直接索引无需冲突检查
        if is_in_place {
            true
        } else {
            let decision =
                conflict_decision.unwrap_or_else(|| prompt_existing_file_decision(&stored_path));
            match decision {
                ExistingFileDecision::Overwrite => true,
                ExistingFileDecision::Cancel => {
                    log_failure(
                        &app_paths.logs,
                        &original_path,
                        "conflict_cancel",
                        &format!("target already exists: {}", stored_path_str),
                    );
                    eprintln!(
                        "\u{23ed}\u{fe0f} 目标文件已存在，已取消导入 [{}]: {}",
                        filename, stored_path_str
                    );
                    return Ok(());
                }
            }
        }
    } else {
        false
    };

    if !is_in_place {
        match move_file(path, &stored_path, should_overwrite) {
            Ok(()) => {
                println!("\u{1f4e6} 文件已移动: {} -> {}", filename, stored_path_str);
            }
            Err(e) => {
                log_failure(&app_paths.logs, &original_path, "rename", &e.to_string());
                eprintln!("\u{26a0}\u{fe0f} 移动文件失败 [{}]: {}", filename, e);
                return Ok(());
            }
        }
    } // end if !is_in_place

    if let Err(e) = db::init_database(&app_paths.db_path) {
        log_failure(&app_paths.logs, &original_path, "db_init", &e.to_string());
        eprintln!("\u{26a0}\u{fe0f} 初始化数据库失败 [{}]: {}", filename, e);
        return Ok(());
    }

    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(e) => {
            log_failure(&app_paths.logs, &original_path, "db_open", &e.to_string());
            eprintln!("\u{26a0}\u{fe0f} 打开数据库失败 [{}]: {}", filename, e);
            return Ok(());
        }
    };

    let input = NewDocument {
        filename,
        original_path: Some(&original_path),
        stored_path: &stored_path_str,
        content: &content,
        folder_type: &classification.folder_type,
        category: &classification.category,
        domain: &classification.domain,
        doc_type: &classification.doc_type,
        file_ext: Some(&extracted.file_ext),
        file_size,
        summary: None,
        tags: None,
        privacy_score: classification.privacy_score,
        risk_level: &classification.risk_level,
        processing_status: "indexed",
        summary_status: "skipped",
    };

    match db::upsert_document(&conn, &input) {
        Ok((true, doc)) => {
            log_success(
                &app_paths.logs,
                &original_path,
                &stored_path_str,
                &doc.folder_type,
                &doc.category,
                &file_hash,
            );
            println!(
                "\u{2705} 处理完成: [{}] folder={} category={} id={}",
                filename, doc.folder_type, doc.category, doc.id
            );
        }
        Ok((false, _)) => {}
        Err(e) => {
            log_failure(&app_paths.logs, &original_path, "db_write", &e.to_string());
            eprintln!(
                "\u{26a0}\u{fe0f} 数据库写入失败 [{}]（文件已在 library 中，id={}）: {}",
                filename, stored_path_str, e
            );
        }
    }

    Ok(())
}

/// 索引已在 library 目录中的文件 — 跳过移动，直接 extract + classify + upsert
/// 索引已在 library 目录中的文件 — extract + classify + upsert
/// 返回 IndexResult 指示文件是否在索引过程中被移动到新路径（watcher 用于更新去重缓存）
pub fn index_file_in_place(path: &Path, app_paths: &AppPaths) -> anyhow::Result<IndexResult> {
    if !is_supported_file(path) {
        return Ok(IndexResult {
            moved_to: None,
            changed: false,
        });
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // 尝试提取文本；失败时创建 failed 状态记录并继续
    let extracted = match extractor::extract_text(path) {
        Ok(e) => e,
        Err(extract_err) => {
            return handle_extraction_failure(path, filename, app_paths, &extract_err);
        }
    };

    let content = extracted.text;
    let file_size = std::fs::metadata(path).ok().map(|m| m.len() as i64);
    let file_hash = db::compute_hash(&content);
    let classification = classify_document(filename, &content);

    let stored_path = build_stored_path(
        &app_paths.library,
        filename,
        &file_hash,
        &classification.folder_type,
    );

    let stored_path_str = stored_path_for_db(&stored_path, app_paths)
        .to_string_lossy()
        .to_string();

    db::init_database(&app_paths.db_path)?;

    // 提前检查数据库：文件已在正确位置且内容未变 → 静默跳过
    if stored_path == path {
        let conn = rusqlite::Connection::open(&app_paths.db_path)?;
        if let Ok(Some(existing)) = db::get_document_by_stored_path(&conn, &stored_path_str)
            && existing.file_hash == file_hash
        {
            return Ok(IndexResult {
                moved_to: None,
                changed: false,
            });
        }
    }

    // 如果文件不在预期子目录中，移动到正确位置
    let moved_to = if stored_path != path {
        if let Some(parent) = stored_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(path, &stored_path)?;
        eprintln!(
            "[watch] 文件已归类: {} -> {}",
            filename,
            stored_path_for_db(&stored_path, app_paths).display()
        );
        Some(stored_path.clone())
    } else {
        None
    };

    let conn = rusqlite::Connection::open(&app_paths.db_path)?;

    let input = NewDocument {
        filename,
        original_path: Some(&path.to_string_lossy()),
        stored_path: &stored_path_str,
        content: &content,
        folder_type: &classification.folder_type,
        category: &classification.category,
        domain: &classification.domain,
        doc_type: &classification.doc_type,
        file_ext: Some(&extracted.file_ext),
        file_size,
        summary: None,
        tags: None,
        privacy_score: classification.privacy_score,
        risk_level: &classification.risk_level,
        processing_status: "indexed",
        summary_status: "skipped",
    };

    let changed = match db::upsert_document(&conn, &input) {
        Ok((true, doc)) => {
            println!(
                "💾 已索引 [{}] id={} folder={} category={}",
                filename, doc.id, doc.folder_type, doc.category
            );
            true
        }
        Ok((false, _)) => {
            // 内容未变，不重复输出
            false
        }
        Err(e) => {
            eprintln!("[watch] 数据库写入失败 [{}]: {}", filename, e);
            false
        }
    };

    Ok(IndexResult { moved_to, changed })
}

/// 文本提取失败时：基于文件名做简化分类，移动文件到正确子目录，创建 failed 状态记录
fn handle_extraction_failure(
    path: &Path,
    filename: &str,
    app_paths: &AppPaths,
    error: &anyhow::Error,
) -> anyhow::Result<IndexResult> {
    // 基于文件名做简化分类（无内容可用，仅依赖文件名中的关键词和扩展名）
    let classification = classify_document(filename, "");
    let file_hash = db::compute_hash("");

    let stored_path = build_stored_path(
        &app_paths.library,
        filename,
        &file_hash,
        &classification.folder_type,
    );

    let stored_path_str = stored_path_for_db(&stored_path, app_paths)
        .to_string_lossy()
        .to_string();

    // 移动文件到正确子目录
    let moved_to = if stored_path != path {
        if let Some(parent) = stored_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(path, &stored_path)?;
        Some(stored_path.clone())
    } else {
        None
    };

    let file_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    if let Err(db_err) = db::init_database(&app_paths.db_path) {
        eprintln!(
            "[watch] 提取失败且无法初始化数据库 [{}]: {} | {}",
            filename, error, db_err
        );
        return Ok(IndexResult {
            moved_to,
            changed: false,
        });
    }

    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(db_err) => {
            eprintln!(
                "[watch] 提取失败且无法打开数据库 [{}]: {} | {}",
                filename, error, db_err
            );
            return Ok(IndexResult {
                moved_to,
                changed: false,
            });
        }
    };

    let input = NewDocument {
        filename,
        original_path: Some(&path.to_string_lossy()),
        stored_path: &stored_path_str,
        content: "",
        folder_type: &classification.folder_type,
        category: &classification.category,
        domain: &classification.domain,
        doc_type: &classification.doc_type,
        file_ext: file_ext.as_deref(),
        file_size: std::fs::metadata(path).ok().map(|m| m.len() as i64),
        summary: None,
        tags: None,
        privacy_score: classification.privacy_score,
        risk_level: &classification.risk_level,
        processing_status: "failed",
        summary_status: "skipped",
    };

    let changed = match db::upsert_document(&conn, &input) {
        Ok((true, _)) => {
            eprintln!(
                "[watch] 提取失败但已创建记录 [{}] (folder={}): {}",
                filename, classification.folder_type, error
            );
            true
        }
        Ok((false, _)) => false,
        Err(db_err) => {
            eprintln!(
                "[watch] 提取失败且数据库写入失败 [{}]: {} | {}",
                filename, error, db_err
            );
            false
        }
    };

    Ok(IndexResult { moved_to, changed })
}

fn stored_path_for_db(path: &Path, app_paths: &AppPaths) -> PathBuf {
    path.strip_prefix(&app_paths.root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn prompt_existing_file_decision(dest: &Path) -> ExistingFileDecision {
    if !io::stdin().is_terminal() {
        return ExistingFileDecision::Cancel;
    }

    loop {
        print!(
            "目标文件已存在: {}。覆盖还是取消？[o]verwrite/[c]ancel: ",
            dest.display()
        );
        let _ = io::stdout().flush();

        let mut answer = String::new();
        match io::stdin().read_line(&mut answer) {
            Ok(0) | Err(_) => return ExistingFileDecision::Cancel,
            Ok(_) => match answer.trim().to_ascii_lowercase().as_str() {
                "o" | "overwrite" | "y" | "yes" => return ExistingFileDecision::Overwrite,
                "c" | "cancel" | "n" | "no" | "" => return ExistingFileDecision::Cancel,
                _ => eprintln!("请输入 o 覆盖，或 c 取消。"),
            },
        }
    }
}

fn move_file(src: &Path, dest: &Path, overwrite: bool) -> io::Result<()> {
    if !overwrite && dest.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("target already exists: {}", dest.display()),
        ));
    }

    match fs::rename(src, dest) {
        Ok(()) => Ok(()),
        Err(err) if overwrite && err.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_file(dest)?;
            move_file(src, dest, false)
        }
        Err(err) if is_cross_device_error(&err) => {
            fs::copy(src, dest)?;
            fs::remove_file(src)
        }
        Err(err) => Err(err),
    }
}

fn is_cross_device_error(err: &io::Error) -> bool {
    #[cfg(unix)]
    const EXDEV: i32 = 18;
    #[cfg(windows)]
    const EXDEV: i32 = 17;
    err.raw_os_error() == Some(EXDEV)
}

fn log_success(
    logs_dir: &Path,
    original_path: &str,
    stored_path: &str,
    folder_type: &str,
    category: &str,
    file_hash: &str,
) {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let hash8 = &file_hash[..8.min(file_hash.len())];
    let line = format!(
        "{} | {} | {} | {} | {} | {}\n",
        now, original_path, stored_path, folder_type, category, hash8
    );
    let log_path = logs_dir.join("imports.log");
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

fn log_failure(logs_dir: &Path, original_path: &str, stage: &str, error: &str) {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let line = format!("{} | {} | {} | {}\n", now, original_path, stage, error);
    let log_path = logs_dir.join("failed_imports.log");
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

fn move_to_quarantine(
    src: &Path,
    quarantine_dir: &Path,
    filename: &str,
    reason: &str,
) -> anyhow::Result<()> {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let safe_name = filename.replace(['/', '\\', '\0'], "_");
    let dest = quarantine_dir.join(format!("{}_{}_{}", date, reason, safe_name));

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(src, &dest)?;
    println!(
        "\u{1f6ab} 失败文件已隔离: {} -> {}",
        filename,
        dest.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let pid = std::process::id();
        std::env::temp_dir().join(format!("omniown_processor_test_{}_{}", pid, name))
    }

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    // --- is_supported_file ---

    #[test]
    fn supported_file_txt() {
        let path = Path::new("readme.txt");
        assert!(is_supported_file(path));
    }

    #[test]
    fn supported_file_md() {
        let path = Path::new("doc.md");
        assert!(is_supported_file(path));
    }

    #[test]
    fn supported_file_html() {
        let path = Path::new("index.html");
        assert!(is_supported_file(path));
    }

    #[test]
    fn supported_file_rs() {
        let path = Path::new("main.rs");
        assert!(is_supported_file(path));
    }

    #[test]
    fn supported_file_case_insensitive() {
        let path = Path::new("Doc.TXT");
        assert!(is_supported_file(path));
        let path = Path::new("ReadMe.MD");
        assert!(is_supported_file(path));
    }

    #[test]
    fn unsupported_file_png() {
        let path = Path::new("image.png");
        assert!(!is_supported_file(path));
    }

    #[test]
    fn unsupported_file_zip() {
        let path = Path::new("archive.zip");
        assert!(!is_supported_file(path));
    }

    #[test]
    fn unsupported_file_no_extension() {
        let path = Path::new("Makefile");
        assert!(!is_supported_file(path));
    }

    // --- stored_path_for_db ---

    #[test]
    fn stored_path_strips_root_prefix() {
        let app_paths = AppPaths::new("/tmp/project");
        let full = PathBuf::from("/tmp/project/library/public/note.md");
        let relative = stored_path_for_db(&full, &app_paths);
        assert_eq!(relative, PathBuf::from("library/public/note.md"));
    }

    #[test]
    fn stored_path_outside_root_returns_full() {
        let app_paths = AppPaths::new("/tmp/project");
        let outside = PathBuf::from("/other/path/file.txt");
        let result = stored_path_for_db(&outside, &app_paths);
        assert_eq!(result, outside);
    }

    // --- move_file ---

    #[test]
    fn move_file_normal() {
        let dir = temp_dir("move_normal");
        fs::create_dir_all(&dir).unwrap();
        let src = create_test_file(&dir, "src.txt", "hello");
        let dst = dir.join("dst.txt");

        assert!(move_file(&src, &dst, false).is_ok());
        assert!(!src.exists());
        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "hello");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn move_file_overwrite_replaces_existing() {
        let dir = temp_dir("move_overwrite");
        fs::create_dir_all(&dir).unwrap();
        let src = create_test_file(&dir, "src.txt", "new content");
        let dst = create_test_file(&dir, "dst.txt", "old content");

        assert!(move_file(&src, &dst, true).is_ok());
        assert!(!src.exists());
        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "new content");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn move_file_no_overwrite_errors_on_existing() {
        let dir = temp_dir("move_no_overwrite");
        fs::create_dir_all(&dir).unwrap();
        let src = create_test_file(&dir, "src.txt", "new");
        let dst = create_test_file(&dir, "dst.txt", "old");

        let result = move_file(&src, &dst, false);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::AlreadyExists);

        fs::remove_dir_all(&dir).ok();
    }

    // --- prompt_existing_file_decision (non-interactive) ---

    #[test]
    fn non_interactive_stdin_returns_cancel() {
        // When stdin is not a terminal, should always return Cancel.
        // We verify this by observing that no actual interactive choice can happen.
        let dst = Path::new("/tmp/nonexistent/file.txt");
        let decision = prompt_existing_file_decision(dst);
        assert_eq!(decision, ExistingFileDecision::Cancel);
    }

    // --- is_cross_device_error ---

    #[test]
    fn cross_device_error_detection() {
        let err = io::Error::from_raw_os_error(18);
        assert!(is_cross_device_error(&err));
    }

    #[test]
    fn non_cross_device_errors() {
        let not_found = io::Error::new(io::ErrorKind::NotFound, "not found");
        assert!(!is_cross_device_error(&not_found));

        let permission = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        assert!(!is_cross_device_error(&permission));
    }

    // --- process_file_with_conflict_decision (integration-style) ---

    #[test]
    fn process_unsupported_file_skips() {
        let root = temp_dir("unsupported");
        let paths = AppPaths::new(&root);
        paths.init_directories().unwrap();
        db::init_database(&paths.db_path).unwrap();

        let png = create_test_file(&paths.public, "doc.png", "fake png");

        let result = process_file_with_conflict_decision(&png, &paths, None);
        assert!(result.is_ok());
        // Unsupported file should remain in place
        assert!(png.exists());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn process_file_successful_import() {
        let root = temp_dir("success");
        let paths = AppPaths::new(&root);
        paths.init_directories().unwrap();
        db::init_database(&paths.db_path).unwrap();

        let txt = create_test_file(&paths.public, "hello.txt", "Hello, world!");

        let result = process_file_with_conflict_decision(&txt, &paths, None);
        assert!(result.is_ok());
        // File already in library/public/ — stays in place, indexed
        // Should be in library/public/
        let stored = paths.public.join("hello.txt");
        assert!(stored.exists());
        assert_eq!(fs::read_to_string(&stored).unwrap(), "Hello, world!");

        // Should be in database
        let conn = rusqlite::Connection::open(&paths.db_path).unwrap();
        let doc =
            db::get_document_by_stored_path(&conn, &format!("library/public/hello.txt")).unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().filename, "hello.txt");

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn process_file_with_overwrite_decision() {
        let root = temp_dir("overwrite_decision");
        let paths = AppPaths::new(&root);
        paths.init_directories().unwrap();
        db::init_database(&paths.db_path).unwrap();

        // Create an existing file in library first
        let existing = paths.public.join("note.txt");
        fs::create_dir_all(&paths.public).unwrap();
        fs::write(&existing, "old content").unwrap();
        // Register it in the database
        {
            let conn = rusqlite::Connection::open(&paths.db_path).unwrap();
            let input = NewDocument {
                filename: "note.txt",
                original_path: Some("library/public/note.txt"),
                stored_path: "library/public/note.txt",
                content: "old content",
                folder_type: "public",
                category: "notes",
                domain: "general",
                doc_type: "note",
                file_ext: Some("txt"),
                file_size: Some(11),
                summary: None,
                tags: None,
                privacy_score: 0.0,
                risk_level: "low",
                processing_status: "indexed",
                summary_status: "skipped",
            };
            db::upsert_document(&conn, &input).unwrap();
        }

        // Now import a new file with same name, decision = Overwrite
        let new_file = create_test_file(&paths.library, "note.txt", "new content");
        let result = process_file_with_conflict_decision(
            &new_file,
            &paths,
            Some(ExistingFileDecision::Overwrite),
        );
        assert!(result.is_ok());

        // Library file should have new content
        let content = fs::read_to_string(&existing).unwrap();
        assert_eq!(content, "new content");

        // DB should have the updated content
        let conn = rusqlite::Connection::open(&paths.db_path).unwrap();
        let doc = db::get_document_by_stored_path(&conn, "library/public/note.txt")
            .unwrap()
            .unwrap();
        assert_eq!(doc.file_hash, db::compute_hash("new content"));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn process_file_with_cancel_decision() {
        let root = temp_dir("cancel_decision");
        let paths = AppPaths::new(&root);
        paths.init_directories().unwrap();
        db::init_database(&paths.db_path).unwrap();

        // Create an existing file in library
        let existing = paths.public.join("note.txt");
        fs::create_dir_all(&paths.public).unwrap();
        fs::write(&existing, "old content").unwrap();
        {
            let conn = rusqlite::Connection::open(&paths.db_path).unwrap();
            let input = NewDocument {
                filename: "note.txt",
                original_path: Some("library/public/note.txt"),
                stored_path: "library/public/note.txt",
                content: "old content",
                folder_type: "public",
                category: "notes",
                domain: "general",
                doc_type: "note",
                file_ext: Some("txt"),
                file_size: Some(11),
                summary: None,
                tags: None,
                privacy_score: 0.0,
                risk_level: "low",
                processing_status: "indexed",
                summary_status: "skipped",
            };
            db::upsert_document(&conn, &input).unwrap();
        }

        // Import with Cancel — inbox file should stay, library file unchanged
        let new_file = create_test_file(&paths.library, "note.txt", "new content");
        let result = process_file_with_conflict_decision(
            &new_file,
            &paths,
            Some(ExistingFileDecision::Cancel),
        );
        assert!(result.is_ok());

        // New file in library root should still exist (not moved to public/)
        assert!(new_file.exists());
        // Library file unchanged
        assert_eq!(fs::read_to_string(&existing).unwrap(), "old content");

        fs::remove_dir_all(&root).ok();
    }

    // --- build_stored_path (from storage) ---

    #[test]
    fn build_stored_path_has_correct_prefix() {
        let path = build_stored_path(
            Path::new("library"),
            "note.md",
            "a81f39c2abcdef1234567890abcdef1234567890",
            "public",
        );
        let s = path.to_string_lossy().to_string();
        assert_eq!(s, "library/public/note.md");
        assert!(!s.contains("a81f39c2"));
    }

    #[test]
    fn private_file_has_private_prefix() {
        let path = build_stored_path(
            Path::new("library"),
            "secret.md",
            "bbbbbbbb0000000000000000000000000000000000",
            "private",
        );
        let s = path.to_string_lossy().to_string();
        assert_eq!(s, "library/private/secret.md");
    }

    #[test]
    fn sanitize_removes_path_separators() {
        let path = build_stored_path(
            Path::new("library"),
            "evil/../../etc/passwd.md",
            "aaaaaaaa0000000000000000000000000000000000",
            "public",
        );
        let s = path.to_string_lossy().to_string();
        assert!(!s.contains("../"));
        assert_eq!(s, "library/public/evil_.._.._etc_passwd.md");
    }

    #[test]
    fn empty_filename_gets_default() {
        let path = build_stored_path(
            Path::new("library"),
            "",
            "aaaaaaaa0000000000000000000000000000000000",
            "public",
        );
        let s = path.to_string_lossy().to_string();
        assert_eq!(s, "library/public/unnamed");
    }

    #[test]
    fn old_library_filename_detection_matches_legacy_names() {
        assert!(is_old_library_filename("2026-05-23_b8184ef2_test.txt"));
        assert!(is_old_library_filename(
            "2026-05-23_c4b391be_AI使用方法.txt"
        ));
        assert!(!is_old_library_filename("test.txt"));
        assert!(!is_old_library_filename("2026-05-23_test.txt"));
        assert!(!is_old_library_filename("2026-05-23_nothexzz_test.txt"));
    }
}
