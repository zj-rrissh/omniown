use crate::classifier::classify_document;
use crate::db::{self, NewDocument};
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

    let stored_path = crate::storage::build_stored_path(
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

    let should_overwrite = if stored_path.exists() {
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
    } else {
        false
    };

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
        embedding_status: "pending",
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
    const EXDEV: i32 = 18;
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
