use crate::classifier::classify_document;
use crate::db::{self, NewDocument};
use crate::fs_layout::AppPaths;
use std::fs;
use std::path::Path;

pub const ALLOWED_EXTENSIONS: &[&str] = &["txt", "md", "rs", "js", "ts", "py", "java", "go", "cpp", "c"];

pub fn process_file(path: &Path, app_paths: &AppPaths) -> anyhow::Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        println!("⏭️ 跳过不支持的文件类型: {:?}", path);
        return Ok(());
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let original_path = path.to_string_lossy().to_string();
    let file_size = fs::metadata(path).ok().map(|m| m.len() as i64);

    let content = fs::read_to_string(path)?;
    let file_hash = db::compute_hash(&content);

    let classification = classify_document(filename, &content);

    let stored_path = crate::storage::build_stored_path(
        filename,
        &file_hash,
        &classification.folder_type,
        &classification.category,
    );

    let stored_path_str = stored_path.to_string_lossy().to_string();

    if let Some(parent) = stored_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(path, &stored_path)?;
    println!("📦 文件已移动: {} -> {}", filename, stored_path_str);

    let conn = rusqlite::Connection::open(&app_paths.db_path)?;

    let input = NewDocument {
        filename,
        original_path: Some(&original_path),
        stored_path: &stored_path_str,
        content: &content,
        folder_type: &classification.folder_type,
        category: &classification.category,
        domain: &classification.domain,
        doc_type: &classification.doc_type,
        file_ext: Some(&ext),
        file_size,
        summary: None,
        tags: None,
        privacy_score: classification.privacy_score,
        risk_level: &classification.risk_level,
        processing_status: "indexed",
        embedding_status: "pending",
        summary_status: "skipped",
    };

    let (changed, doc) = db::upsert_document(&conn, &input)?;

    if changed {
        println!(
            "✅ 处理完成: [{}] folder={} category={} id={}",
            filename, doc.folder_type, doc.category, doc.id
        );
    }

    Ok(())
}
