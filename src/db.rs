use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::path::Path;

// ---- 数据模型 ----

#[derive(Debug, Clone)]
pub struct Document {
    pub id: i64,
    pub filename: String,
    pub original_path: Option<String>,
    pub stored_path: String,
    pub file_ext: Option<String>,
    pub file_size: Option<i64>,
    pub file_hash: String,
    pub folder_type: String,
    pub category: String,
    pub domain: String,
    pub doc_type: String,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub tags: Option<String>,
    pub privacy_score: f64,
    pub risk_level: String,
    pub processing_status: String,
    pub embedding_status: String,
    pub summary_status: String,
    pub created_at: String,
    pub updated_at: String,
    pub imported_at: String,
}

pub struct NewDocument<'a> {
    pub filename: &'a str,
    pub original_path: Option<&'a str>,
    pub stored_path: &'a str,
    pub content: &'a str,
    pub folder_type: &'a str,
    pub category: &'a str,
    pub domain: &'a str,
    pub doc_type: &'a str,
    pub file_ext: Option<&'a str>,
    pub file_size: Option<i64>,
    pub summary: Option<&'a str>,
    pub tags: Option<&'a str>,
    pub privacy_score: f64,
    pub risk_level: &'a str,
    pub processing_status: &'a str,
    pub embedding_status: &'a str,
    pub summary_status: &'a str,
}

// ---- 辅助函数 ----

pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher.finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

fn row_to_doc(row: &rusqlite::Row) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get(0)?,
        filename: row.get(1)?,
        original_path: row.get(2)?,
        stored_path: row.get(3)?,
        file_ext: row.get(4)?,
        file_size: row.get(5)?,
        file_hash: row.get(6)?,
        folder_type: row.get(7)?,
        category: row.get(8)?,
        domain: row.get(9)?,
        doc_type: row.get(10)?,
        content: row.get(11)?,
        summary: row.get(12)?,
        tags: row.get(13)?,
        privacy_score: row.get(14)?,
        risk_level: row.get(15)?,
        processing_status: row.get(16)?,
        embedding_status: row.get(17)?,
        summary_status: row.get(18)?,
        created_at: row.get(19)?,
        updated_at: row.get(20)?,
        imported_at: row.get(21)?,
    })
}

const DOCUMENT_COLUMNS: &str =
    "id, filename, original_path, stored_path, file_ext, file_size, file_hash, \
     folder_type, category, domain, doc_type, content, summary, tags, \
     privacy_score, risk_level, processing_status, embedding_status, summary_status, \
     created_at, updated_at, imported_at";

const DOCUMENT_COLUMNS_NO_CONTENT: &str =
    "id, filename, original_path, stored_path, file_ext, file_size, file_hash, \
     folder_type, category, domain, doc_type, NULL AS content, summary, tags, \
     privacy_score, risk_level, processing_status, embedding_status, summary_status, \
     created_at, updated_at, imported_at";

// ---- CRUD ----

pub fn get_document_by_stored_path(
    conn: &Connection,
    stored_path: &str,
) -> rusqlite::Result<Option<Document>> {
    let sql = format!(
        "SELECT {} FROM documents WHERE stored_path = ?1",
        DOCUMENT_COLUMNS
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![stored_path], row_to_doc)?;
    match rows.next() {
        Some(Ok(doc)) => Ok(Some(doc)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn get_document_by_id(conn: &Connection, id: i64) -> rusqlite::Result<Option<Document>> {
    let sql = format!(
        "SELECT {} FROM documents WHERE id = ?1",
        DOCUMENT_COLUMNS
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![id], row_to_doc)?;
    match rows.next() {
        Some(Ok(doc)) => Ok(Some(doc)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn list_documents_meta(conn: &Connection) -> rusqlite::Result<Vec<Document>> {
    let sql = format!(
        "SELECT {} FROM documents ORDER BY updated_at DESC",
        DOCUMENT_COLUMNS_NO_CONTENT
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_doc)?;
    rows.collect()
}

pub fn list_by_folder_type(
    conn: &Connection,
    folder_type: &str,
) -> rusqlite::Result<Vec<Document>> {
    let sql = format!(
        "SELECT {} FROM documents WHERE folder_type = ?1 ORDER BY updated_at DESC",
        DOCUMENT_COLUMNS_NO_CONTENT
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![folder_type], row_to_doc)?;
    rows.collect()
}

pub fn list_by_category(conn: &Connection, category: &str) -> rusqlite::Result<Vec<Document>> {
    let sql = format!(
        "SELECT {} FROM documents WHERE category = ?1 ORDER BY updated_at DESC",
        DOCUMENT_COLUMNS_NO_CONTENT
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![category], row_to_doc)?;
    rows.collect()
}

pub fn list_pending_embeddings(
    conn: &Connection,
    limit: i64,
) -> rusqlite::Result<Vec<Document>> {
    let sql = format!(
        "SELECT {} FROM documents WHERE embedding_status = 'pending' ORDER BY id LIMIT ?1",
        DOCUMENT_COLUMNS
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![limit], row_to_doc)?;
    rows.collect()
}

pub fn mark_embedding_done(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
    let affected = conn.execute(
        "UPDATE documents SET embedding_status = 'done', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
        params![id],
    )?;
    Ok(affected > 0)
}

pub fn mark_processing_failed(conn: &Connection, stored_path: &str) -> rusqlite::Result<bool> {
    let affected = conn.execute(
        "UPDATE documents SET processing_status = 'failed', updated_at = CURRENT_TIMESTAMP WHERE stored_path = ?1",
        params![stored_path],
    )?;
    Ok(affected > 0)
}

pub fn delete_document_by_stored_path(
    conn: &Connection,
    stored_path: &str,
) -> rusqlite::Result<bool> {
    let affected = conn.execute(
        "DELETE FROM documents WHERE stored_path = ?1",
        params![stored_path],
    )?;
    Ok(affected > 0)
}

pub fn delete_document_by_id(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
    let affected = conn.execute("DELETE FROM documents WHERE id = ?1", params![id])?;
    Ok(affected > 0)
}

pub fn upsert_document(
    conn: &Connection,
    input: &NewDocument,
) -> rusqlite::Result<(bool, Document)> {
    let new_hash = compute_hash(input.content);

    if let Some(existing) = get_document_by_stored_path(conn, input.stored_path)?
        && existing.file_hash == new_hash
    {
        println!("⏩ 文件 [{}] 内容未变，跳过更新", input.filename);
        return Ok((false, existing));
    }

    conn.execute(
        "INSERT INTO documents (
            filename, original_path, stored_path, file_ext, file_size, file_hash,
            folder_type, category, domain, doc_type, content, summary, tags,
            privacy_score, risk_level, processing_status, embedding_status, summary_status,
            updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?18,
            CURRENT_TIMESTAMP
        ) ON CONFLICT(stored_path) DO UPDATE SET
            filename = excluded.filename,
            original_path = excluded.original_path,
            file_ext = excluded.file_ext,
            file_size = excluded.file_size,
            file_hash = excluded.file_hash,
            folder_type = excluded.folder_type,
            category = excluded.category,
            domain = excluded.domain,
            doc_type = excluded.doc_type,
            content = excluded.content,
            summary = excluded.summary,
            tags = excluded.tags,
            privacy_score = excluded.privacy_score,
            risk_level = excluded.risk_level,
            processing_status = excluded.processing_status,
            embedding_status = excluded.embedding_status,
            summary_status = excluded.summary_status,
            updated_at = CURRENT_TIMESTAMP",
        params![
            input.filename,
            input.original_path,
            input.stored_path,
            input.file_ext,
            input.file_size,
            new_hash,
            input.folder_type,
            input.category,
            input.domain,
            input.doc_type,
            input.content,
            input.summary,
            input.tags,
            input.privacy_score,
            input.risk_level,
            input.processing_status,
            input.embedding_status,
            input.summary_status,
        ],
    )?;

    println!("💾 已将 [{}] 的最新状态写入数据库", input.filename);
    let doc = get_document_by_stored_path(conn, input.stored_path)?
        .expect("刚 upsert 的文档应能回读");
    Ok((true, doc))
}

// ---- 数据库初始化 ----

pub fn init_database(db_path: &Path) -> rusqlite::Result<()> {
    println!("🗄️ 正在初始化系统基础设施...");

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("无法创建数据库目录");
    }

    let conn = Connection::open(db_path)?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS documents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            filename TEXT NOT NULL,
            original_path TEXT,
            stored_path TEXT NOT NULL UNIQUE,
            file_ext TEXT,
            file_size INTEGER,
            file_hash TEXT NOT NULL,
            folder_type TEXT NOT NULL DEFAULT 'public',
            category TEXT NOT NULL DEFAULT 'misc',
            domain TEXT NOT NULL DEFAULT 'unknown',
            doc_type TEXT NOT NULL DEFAULT 'unknown',
            content TEXT,
            summary TEXT,
            tags TEXT,
            privacy_score REAL DEFAULT 0,
            risk_level TEXT DEFAULT 'low',
            processing_status TEXT NOT NULL DEFAULT 'pending',
            embedding_status TEXT NOT NULL DEFAULT 'pending',
            summary_status TEXT NOT NULL DEFAULT 'skipped',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            imported_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_documents_hash ON documents(file_hash)",
        "CREATE INDEX IF NOT EXISTS idx_documents_folder_type ON documents(folder_type)",
        "CREATE INDEX IF NOT EXISTS idx_documents_category ON documents(category)",
        "CREATE INDEX IF NOT EXISTS idx_documents_processing_status ON documents(processing_status)",
        "CREATE INDEX IF NOT EXISTS idx_documents_embedding_status ON documents(embedding_status)",
        "CREATE INDEX IF NOT EXISTS idx_documents_updated_at ON documents(updated_at)",
    ];

    for idx in indexes {
        conn.execute(idx, [])?;
    }

    println!("✅ omniown.db 初始化完成，数据表结构已就绪。\n");
    Ok(())
}

// ---- 测试 ----

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL,
                original_path TEXT,
                stored_path TEXT NOT NULL UNIQUE,
                file_ext TEXT,
                file_size INTEGER,
                file_hash TEXT NOT NULL,
                folder_type TEXT NOT NULL DEFAULT 'public',
                category TEXT NOT NULL DEFAULT 'misc',
                domain TEXT NOT NULL DEFAULT 'unknown',
                doc_type TEXT NOT NULL DEFAULT 'unknown',
                content TEXT,
                summary TEXT,
                tags TEXT,
                privacy_score REAL DEFAULT 0,
                risk_level TEXT DEFAULT 'low',
                processing_status TEXT NOT NULL DEFAULT 'pending',
                embedding_status TEXT NOT NULL DEFAULT 'pending',
                summary_status TEXT NOT NULL DEFAULT 'skipped',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                imported_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .unwrap();
        conn
    }

    fn make_input<'a>(
        filename: &'a str,
        stored_path: &'a str,
        content: &'a str,
    ) -> NewDocument<'a> {
        NewDocument {
            filename,
            original_path: None,
            stored_path,
            content,
            folder_type: "public",
            category: "notes",
            domain: "unknown",
            doc_type: "markdown",
            file_ext: Some("md"),
            file_size: Some(content.len() as i64),
            summary: None,
            tags: None,
            privacy_score: 0.1,
            risk_level: "low",
            processing_status: "indexed",
            embedding_status: "pending",
            summary_status: "skipped",
        }
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
    fn upsert_inserts_new_document() {
        let conn = setup_db();
        let input = make_input("test.md", "library/public/notes/test.md", "# Hello");
        let (changed, doc) = upsert_document(&conn, &input).unwrap();
        assert!(changed);
        assert_eq!(doc.filename, "test.md");
        assert_eq!(doc.stored_path, "library/public/notes/test.md");
        assert_eq!(doc.folder_type, "public");
        assert_eq!(doc.category, "notes");
        assert_eq!(doc.content.as_deref(), Some("# Hello"));
        assert_eq!(doc.processing_status, "indexed");
    }

    #[test]
    fn upsert_updates_changed_content() {
        let conn = setup_db();
        let input1 = make_input("u.md", "library/public/notes/u.md", "v1");
        upsert_document(&conn, &input1).unwrap();

        let input2 = make_input("u.md", "library/public/notes/u.md", "v2");
        let (changed, doc) = upsert_document(&conn, &input2).unwrap();
        assert!(changed);
        assert_eq!(doc.content.as_deref(), Some("v2"));
    }

    #[test]
    fn upsert_skips_unchanged_content() {
        let conn = setup_db();
        let input = make_input("same.md", "library/public/notes/same.md", "same");
        upsert_document(&conn, &input).unwrap();
        let (changed, _) = upsert_document(&conn, &input).unwrap();
        assert!(!changed);
    }

    #[test]
    fn different_stored_path_same_hash_allowed() {
        let conn = setup_db();
        let input1 = make_input("a.md", "library/public/notes/a.md", "same content");
        let (changed1, _) = upsert_document(&conn, &input1).unwrap();
        assert!(changed1);

        let input2 = make_input("b.md", "library/public/notes/b.md", "same content");
        let (changed2, _) = upsert_document(&conn, &input2).unwrap();
        assert!(changed2);

        let docs = list_documents_meta(&conn).unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn duplicate_stored_path_triggers_update() {
        let conn = setup_db();
        let input = make_input("dup.md", "library/public/notes/dup.md", "content");
        upsert_document(&conn, &input).unwrap();

        let input2 = NewDocument {
            filename: "other.md",
            ..make_input("other.md", "library/public/notes/dup.md", "other")
        };
        let (changed, doc) = upsert_document(&conn, &input2).unwrap();
        assert!(changed);
        assert_eq!(doc.filename, "other.md");
    }

    #[test]
    fn get_by_stored_path_found() {
        let conn = setup_db();
        let input = make_input("find.md", "library/public/code/find.md", "code");
        upsert_document(&conn, &input).unwrap();
        let doc = get_document_by_stored_path(&conn, "library/public/code/find.md")
            .unwrap()
            .unwrap();
        assert_eq!(doc.filename, "find.md");
    }

    #[test]
    fn get_by_stored_path_not_found() {
        let conn = setup_db();
        let result = get_document_by_stored_path(&conn, "nonexistent/path.md").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_document_by_id_found() {
        let conn = setup_db();
        let input = make_input("by-id.md", "library/public/notes/by-id.md", "content");
        let (_, doc) = upsert_document(&conn, &input).unwrap();
        let fetched = get_document_by_id(&conn, doc.id).unwrap().unwrap();
        assert_eq!(fetched.filename, "by-id.md");
    }

    #[test]
    fn list_documents_meta_returns_all_without_content() {
        let conn = setup_db();
        let input1 = make_input("a.md", "library/public/notes/a.md", "A");
        let input2 = make_input("b.md", "library/public/docs/b.md", "B");
        upsert_document(&conn, &input1).unwrap();
        upsert_document(&conn, &input2).unwrap();
        let docs = list_documents_meta(&conn).unwrap();
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().all(|d| d.content.is_none()));
    }

    #[test]
    fn list_by_folder_type_filters() {
        let conn = setup_db();
        let pub_input = make_input("pub.md", "library/public/notes/pub.md", "pub");
        upsert_document(&conn, &pub_input).unwrap();

        let priv_input = NewDocument {
            filename: "priv.md",
            stored_path: "library/private/finance/priv.md",
            folder_type: "private",
            category: "finance",
            domain: "finance",
            doc_type: "markdown",
            privacy_score: 0.9,
            risk_level: "medium",
            ..make_input("priv.md", "library/private/finance/priv.md", "priv")
        };
        upsert_document(&conn, &priv_input).unwrap();

        assert_eq!(list_by_folder_type(&conn, "public").unwrap().len(), 1);
        assert_eq!(list_by_folder_type(&conn, "private").unwrap().len(), 1);
    }

    #[test]
    fn list_by_category_filters() {
        let conn = setup_db();
        let notes = make_input("n.md", "library/public/notes/n.md", "notes");
        upsert_document(&conn, &notes).unwrap();

        let code_input = NewDocument {
            filename: "c.rs",
            stored_path: "library/public/code/c.rs",
            category: "code",
            domain: "dev",
            doc_type: "code",
            file_ext: Some("rs"),
            ..make_input("c.rs", "library/public/code/c.rs", "fn main() {}")
        };
        upsert_document(&conn, &code_input).unwrap();

        assert_eq!(list_by_category(&conn, "notes").unwrap().len(), 1);
        assert_eq!(list_by_category(&conn, "code").unwrap().len(), 1);
        assert_eq!(list_by_category(&conn, "finance").unwrap().len(), 0);
    }

    #[test]
    fn list_pending_embeddings_and_mark_done() {
        let conn = setup_db();
        let input = make_input("e1.md", "library/public/notes/e1.md", "embed me");
        let (_, doc) = upsert_document(&conn, &input).unwrap();

        let pending = list_pending_embeddings(&conn, 10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].embedding_status, "pending");

        assert!(mark_embedding_done(&conn, doc.id).unwrap());
        assert_eq!(list_pending_embeddings(&conn, 10).unwrap().len(), 0);
    }

    #[test]
    fn mark_processing_failed_sets_status() {
        let conn = setup_db();
        let input = make_input("fail.md", "library/public/notes/fail.md", "bad");
        upsert_document(&conn, &input).unwrap();

        assert!(mark_processing_failed(&conn, "library/public/notes/fail.md").unwrap());
        let doc = get_document_by_stored_path(&conn, "library/public/notes/fail.md")
            .unwrap()
            .unwrap();
        assert_eq!(doc.processing_status, "failed");
    }

    #[test]
    fn delete_document_by_stored_path_removes() {
        let conn = setup_db();
        let input = make_input("del.md", "library/public/notes/del.md", "x");
        upsert_document(&conn, &input).unwrap();

        assert!(delete_document_by_stored_path(&conn, "library/public/notes/del.md").unwrap());
        assert!(
            get_document_by_stored_path(&conn, "library/public/notes/del.md")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn delete_nonexistent_stored_path_returns_false() {
        let conn = setup_db();
        assert!(!delete_document_by_stored_path(&conn, "ghost/path.md").unwrap());
        assert!(!delete_document_by_id(&conn, 999).unwrap());
    }

    #[test]
    fn delete_document_by_id_removes() {
        let conn = setup_db();
        let input = make_input("delid.md", "library/public/notes/delid.md", "x");
        let (_, doc) = upsert_document(&conn, &input).unwrap();

        assert!(delete_document_by_id(&conn, doc.id).unwrap());
        assert!(get_document_by_id(&conn, doc.id).unwrap().is_none());
    }

    #[test]
    fn mark_embedding_done_nonexistent_returns_false() {
        let conn = setup_db();
        assert!(!mark_embedding_done(&conn, 99999).unwrap());
    }
}
