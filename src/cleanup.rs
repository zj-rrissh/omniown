use crate::db;
use crate::fs_layout::AppPaths;
use crate::storage;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CleanupReport {
    pub files_deleted: usize,
    pub db_records_deleted: usize,
}

pub fn cleanup_old_library_documents(app_paths: &AppPaths) -> anyhow::Result<CleanupReport> {
    let mut report = CleanupReport::default();

    for folder in [&app_paths.public, &app_paths.private] {
        report.files_deleted += delete_old_library_files(folder)?;
    }

    if app_paths.db_path.exists() {
        let conn = Connection::open(&app_paths.db_path)?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        for stored_path in list_old_library_document_paths(&conn)? {
            if db::delete_document_by_stored_path(&conn, &stored_path)? {
                report.db_records_deleted += 1;
            }
        }
    }

    Ok(report)
}

fn delete_old_library_files(folder: &Path) -> anyhow::Result<usize> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e.into()),
    };

    let mut deleted = 0;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if storage::is_old_library_filename(name) {
            fs::remove_file(&path)?;
            deleted += 1;
        }
    }

    Ok(deleted)
}

fn list_old_library_document_paths(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT stored_path FROM documents")?;
    let paths = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut old_paths = Vec::new();
    for path in paths {
        let path = path?;
        if is_old_library_stored_path(&path) {
            old_paths.push(path);
        }
    }
    Ok(old_paths)
}

fn is_old_library_stored_path(stored_path: &str) -> bool {
    let path = PathBuf::from(stored_path);
    let mut components = path.components().filter_map(|c| c.as_os_str().to_str());

    matches!(components.next(), Some("library"))
        && matches!(components.next(), Some("public" | "private"))
        && components
            .next()
            .is_some_and(storage::is_old_library_filename)
        && components.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NewDocument;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_PROJECT_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn make_temp_project() -> (AppPaths, PathBuf) {
        let counter = TEMP_PROJECT_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "omniown_cleanup_test_{}_{}",
            std::process::id(),
            counter
        ));
        fs::create_dir_all(&root).unwrap();

        let app_paths = AppPaths::new(&root);
        app_paths.init_directories().unwrap();
        db::init_database(&app_paths.db_path).unwrap();

        (app_paths, root)
    }

    fn write_file(path: &Path, content: &str) {
        let mut f = fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    fn insert_doc(conn: &Connection, filename: &str, stored_path: &str) {
        let input = NewDocument {
            filename,
            original_path: Some("inbox/source.md"),
            stored_path,
            content: "hello",
            folder_type: "public",
            category: "notes",
            domain: "general",
            doc_type: "markdown",
            file_ext: Some("md"),
            file_size: Some(5),
            summary: None,
            tags: None,
            privacy_score: 0.0,
            risk_level: "low",
            processing_status: "indexed",
            embedding_status: "pending",
            summary_status: "skipped",
        };
        db::upsert_document(conn, &input).unwrap();
    }

    #[test]
    fn cleanup_deletes_old_files_and_old_db_records() {
        let (app_paths, root) = make_temp_project();
        let old_path = app_paths.public.join("2026-05-23_b8184ef2_test.md");
        let new_path = app_paths.public.join("test.md");
        write_file(&old_path, "old");
        write_file(&new_path, "new");

        let conn = Connection::open(&app_paths.db_path).unwrap();
        insert_doc(
            &conn,
            "old.md",
            "library/public/2026-05-23_b8184ef2_test.md",
        );
        insert_doc(&conn, "new.md", "library/public/test.md");

        let report = cleanup_old_library_documents(&app_paths).unwrap();
        assert_eq!(report.files_deleted, 1);
        assert_eq!(report.db_records_deleted, 1);
        assert!(!old_path.exists());
        assert!(new_path.exists());
        assert_eq!(db::count_documents(&conn).unwrap(), 1);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn cleanup_does_not_delete_new_style_files() {
        let (app_paths, root) = make_temp_project();
        let path = app_paths.private.join("secret.txt");
        write_file(&path, "secret");

        let report = cleanup_old_library_documents(&app_paths).unwrap();
        assert_eq!(report, CleanupReport::default());
        assert!(path.exists());

        fs::remove_dir_all(&root).ok();
    }
}
