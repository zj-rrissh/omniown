use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

// ---- 数据模型 ----

#[derive(Debug, Clone)]
pub struct Document {
    pub id: i64,
    pub filename: String,
    pub file_hash: String,
    pub folder_type: String,
    pub content: Option<String>,
    pub tags: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ---- 辅助函数 ----

fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

fn row_to_doc(row: &rusqlite::Row) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get(0)?,
        filename: row.get(1)?,
        file_hash: row.get(2)?,
        folder_type: row.get(3)?,
        content: row.get(4)?,
        tags: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

// ---- CRUD ----

pub fn insert_document(conn: &Connection, filename: &str, content: &str) -> rusqlite::Result<Document> {
    let file_hash = compute_hash(content);
    conn.execute(
        "INSERT INTO documents (filename, file_hash, folder_type, content) VALUES (?1, ?2, 'public', ?3)",
        params![filename, file_hash, content],
    )?;
    get_document(conn, filename)?.ok_or_else(|| {
        rusqlite::Error::InvalidParameterName("插入后未能回读文档".into())
    })
}

pub fn get_document(conn: &Connection, filename: &str) -> rusqlite::Result<Option<Document>> {
    let mut stmt = conn.prepare(
        "SELECT id, filename, file_hash, folder_type, content, tags, created_at, updated_at \
         FROM documents WHERE filename = ?1",
    )?;
    let mut rows = stmt.query_map(params![filename], row_to_doc)?;
    match rows.next() {
        Some(Ok(doc)) => Ok(Some(doc)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn get_document_by_id(conn: &Connection, id: i64) -> rusqlite::Result<Option<Document>> {
    let mut stmt = conn.prepare(
        "SELECT id, filename, file_hash, folder_type, content, tags, created_at, updated_at \
         FROM documents WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], row_to_doc)?;
    match rows.next() {
        Some(Ok(doc)) => Ok(Some(doc)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn list_documents_meta(conn: &Connection) -> rusqlite::Result<Vec<Document>> {
    let mut stmt = conn.prepare(
        "SELECT id, filename, file_hash, folder_type, NULL AS content, tags, created_at, updated_at \
         FROM documents ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_doc)?;
    rows.collect()
}

pub fn update_document(conn: &Connection, filename: &str, content: &str) -> rusqlite::Result<Option<Document>> {
    let file_hash = compute_hash(content);
    let affected = conn.execute(
        "UPDATE documents SET file_hash = ?1, content = ?2, updated_at = CURRENT_TIMESTAMP WHERE filename = ?3",
        params![file_hash, content, filename],
    )?;
    if affected == 0 {
        return Ok(None);
    }
    get_document(conn, filename)
}

pub fn delete_document_by_id(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
    let affected = conn.execute("DELETE FROM documents WHERE id = ?1", params![id])?;
    Ok(affected > 0)
}

pub fn delete_document(conn: &Connection, filename: &str) -> rusqlite::Result<bool> {
    let affected = conn.execute("DELETE FROM documents WHERE filename = ?1", params![filename])?;
    Ok(affected > 0)
}

pub fn upsert_document(conn: &Connection, filename: &str, content: &str) -> rusqlite::Result<(bool, Document)> {
    let new_hash = compute_hash(content);

    if let Some(existing) = get_document(conn, filename)? {
        if existing.file_hash == new_hash {
            println!("⏩ 文件 [{}] 内容未变，跳过更新", filename);
            return Ok((false, existing));
        }
    }

    conn.execute(
        "INSERT INTO documents (filename, file_hash, folder_type, content, updated_at) \
         VALUES (?1, ?2, 'public', ?3, CURRENT_TIMESTAMP) \
         ON CONFLICT(filename) DO UPDATE SET \
            file_hash = excluded.file_hash,
            content = excluded.content,
            updated_at = CURRENT_TIMESTAMP",
        params![filename, new_hash, content],
    )?;

    println!("💾 已将 [{}] 的最新状态写入数据库", filename);
    let doc = get_document(conn, filename)?.expect("刚 upsert 的文档应能回读");
    Ok((true, doc))
}

// ---- 数据库初始化 ----

pub fn init_database() -> rusqlite::Result<()> {
    println!("🗄️ 正在初始化系统基础设施...");

    let conn = Connection::open("omniown.db")?;

    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS documents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            filename TEXT NOT NULL UNIQUE,
            file_hash TEXT NOT NULL,
            folder_type TEXT NOT NULL,
            content TEXT,
            tags TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    println!("✅ omniown.db 初始化完成，数据表结构已就绪。\n");
    Ok(())
}

// ---- 测试 ----

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL UNIQUE,
                file_hash TEXT NOT NULL,
                folder_type TEXT NOT NULL,
                content TEXT,
                tags TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).unwrap();
        conn
    }

    #[test]
    fn compute_hash_same_content_produces_same_hash() {
        let h1 = compute_hash("hello");
        let h2 = compute_hash("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn compute_hash_different_content_produces_different_hash() {
        let h1 = compute_hash("hello");
        let h2 = compute_hash("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn insert_and_get_document() {
        let conn = setup_db();
        let doc = insert_document(&conn, "test.md", "# Hello").unwrap();
        assert_eq!(doc.filename, "test.md");
        assert_eq!(doc.file_hash, compute_hash("# Hello"));

        let fetched = get_document(&conn, "test.md").unwrap().unwrap();
        assert_eq!(fetched.id, doc.id);
        assert_eq!(fetched.content.as_deref(), Some("# Hello"));
    }

    #[test]
    fn insert_duplicate_filename_fails() {
        let conn = setup_db();
        insert_document(&conn, "dup.md", "content").unwrap();
        let result = insert_document(&conn, "dup.md", "different");
        assert!(result.is_err());
    }

    #[test]
    fn get_document_not_found() {
        let conn = setup_db();
        let result = get_document(&conn, "nonexistent.md").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_document_by_id_found() {
        let conn = setup_db();
        let doc = insert_document(&conn, "by-id.md", "content").unwrap();
        let fetched = get_document_by_id(&conn, doc.id).unwrap().unwrap();
        assert_eq!(fetched.filename, "by-id.md");
    }

    #[test]
    fn list_documents_meta_returns_all() {
        let conn = setup_db();
        insert_document(&conn, "a.md", "A").unwrap();
        insert_document(&conn, "b.md", "B").unwrap();
        let docs = list_documents_meta(&conn).unwrap();
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().all(|d| d.content.is_none()));
    }

    #[test]
    fn update_existing_document() {
        let conn = setup_db();
        insert_document(&conn, "update.md", "v1").unwrap();
        let updated = update_document(&conn, "update.md", "v2").unwrap().unwrap();
        assert_eq!(updated.file_hash, compute_hash("v2"));
        assert_eq!(updated.content.as_deref(), Some("v2"));
    }

    #[test]
    fn update_nonexistent_returns_none() {
        let conn = setup_db();
        let result = update_document(&conn, "ghost.md", "data").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_document_by_id_removes() {
        let conn = setup_db();
        let doc = insert_document(&conn, "del.md", "x").unwrap();
        assert!(delete_document_by_id(&conn, doc.id).unwrap());
        assert!(get_document_by_id(&conn, doc.id).unwrap().is_none());
    }

    #[test]
    fn delete_document_by_filename_removes() {
        let conn = setup_db();
        insert_document(&conn, "del.md", "x").unwrap();
        assert!(delete_document(&conn, "del.md").unwrap());
        assert!(get_document(&conn, "del.md").unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let conn = setup_db();
        assert!(!delete_document(&conn, "nope.md").unwrap());
        assert!(!delete_document_by_id(&conn, 999).unwrap());
    }

    #[test]
    fn upsert_inserts_new() {
        let conn = setup_db();
        let (changed, doc) = upsert_document(&conn, "new.md", "data").unwrap();
        assert!(changed);
        assert_eq!(doc.filename, "new.md");
    }

    #[test]
    fn upsert_updates_changed_content() {
        let conn = setup_db();
        upsert_document(&conn, "u.md", "v1").unwrap();
        let (changed, doc) = upsert_document(&conn, "u.md", "v2").unwrap();
        assert!(changed);
        assert_eq!(doc.content.as_deref(), Some("v2"));
    }

    #[test]
    fn upsert_skips_unchanged_content() {
        let conn = setup_db();
        upsert_document(&conn, "u.md", "same").unwrap();
        let (changed, _) = upsert_document(&conn, "u.md", "same").unwrap();
        assert!(!changed);
    }
}
