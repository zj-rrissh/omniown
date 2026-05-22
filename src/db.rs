use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::path::Path;

// ---- 数据模型 ----

#[allow(dead_code)]
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
    hasher
        .finalize()
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

const DOCUMENT_COLUMNS: &str = "id, filename, original_path, stored_path, file_ext, file_size, file_hash, \
     folder_type, category, domain, doc_type, content, summary, tags, \
     privacy_score, risk_level, processing_status, embedding_status, summary_status, \
     created_at, updated_at, imported_at";

#[allow(dead_code)]
const DOCUMENT_COLUMNS_NO_CONTENT: &str = "id, filename, original_path, stored_path, file_ext, file_size, file_hash, \
     folder_type, category, domain, doc_type, NULL AS content, summary, tags, \
     privacy_score, risk_level, processing_status, embedding_status, summary_status, \
     created_at, updated_at, imported_at";

// ---- CRUD ----

#[allow(dead_code)]
pub fn get_document_by_id(conn: &Connection, id: i64) -> rusqlite::Result<Option<Document>> {
    let sql = format!("SELECT {} FROM documents WHERE id = ?1", DOCUMENT_COLUMNS);
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![id], row_to_doc)?;
    match rows.next() {
        Some(Ok(doc)) => Ok(Some(doc)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

#[allow(dead_code)]
pub fn list_documents_meta(conn: &Connection) -> rusqlite::Result<Vec<Document>> {
    let sql = format!(
        "SELECT {} FROM documents ORDER BY updated_at DESC",
        DOCUMENT_COLUMNS_NO_CONTENT
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_doc)?;
    rows.collect()
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn list_by_category(conn: &Connection, category: &str) -> rusqlite::Result<Vec<Document>> {
    let sql = format!(
        "SELECT {} FROM documents WHERE category = ?1 ORDER BY updated_at DESC",
        DOCUMENT_COLUMNS_NO_CONTENT
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![category], row_to_doc)?;
    rows.collect()
}

#[allow(dead_code)]
pub fn list_pending_embeddings(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<Document>> {
    let sql = format!(
        "SELECT {} FROM documents WHERE embedding_status = 'pending' ORDER BY id LIMIT ?1",
        DOCUMENT_COLUMNS
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![limit], row_to_doc)?;
    rows.collect()
}

#[allow(dead_code)]
pub fn mark_embedding_done(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
    let affected = conn.execute(
        "UPDATE documents SET embedding_status = 'done', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
        params![id],
    )?;
    Ok(affected > 0)
}

#[allow(dead_code)]
pub fn mark_processing_failed(conn: &Connection, stored_path: &str) -> rusqlite::Result<bool> {
    let affected = conn.execute(
        "UPDATE documents SET processing_status = 'failed', updated_at = CURRENT_TIMESTAMP WHERE stored_path = ?1",
        params![stored_path],
    )?;
    Ok(affected > 0)
}

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

#[allow(dead_code)]
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
    let doc =
        get_document_by_stored_path(conn, input.stored_path)?.expect("刚 upsert 的文档应能回读");
    Ok((true, doc))
}

// ---- 统计查询 ----

pub fn count_documents(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM documents", [], |r| r.get(0))
}

pub fn count_by_folder_type(conn: &Connection, folder_type: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE folder_type = ?1",
        params![folder_type],
        |r| r.get(0),
    )
}

#[allow(dead_code)]
pub fn count_by_category(conn: &Connection, category: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE category = ?1",
        params![category],
        |r| r.get(0),
    )
}

pub fn count_by_processing_status(conn: &Connection, status: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE processing_status = ?1",
        params![status],
        |r| r.get(0),
    )
}

pub fn count_embeddings(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM document_embeddings", [], |r| r.get(0))
}

pub fn count_pending_embeddings(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM documents WHERE embedding_status = 'pending'",
        [],
        |r| r.get(0),
    )
}

// ---- 数据库初始化 ----

pub fn init_database(db_path: &Path) -> rusqlite::Result<()> {
    println!("🗄️ 正在初始化系统基础设施...");

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).expect("无法创建数据库目录");
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

    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
            filename,
            content,
            tags,
            summary,
            content='documents',
            content_rowid='id'
        )",
        [],
    )?;

    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
            INSERT INTO documents_fts(rowid, filename, content, tags, summary)
            VALUES (new.id, new.filename, new.content, new.tags, new.summary);
        END;

        CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
            INSERT INTO documents_fts(documents_fts, rowid, filename, content, tags, summary)
            VALUES('delete', old.id, old.filename, old.content, old.tags, old.summary);
        END;

        CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE ON documents BEGIN
            INSERT INTO documents_fts(documents_fts, rowid, filename, content, tags, summary)
            VALUES('delete', old.id, old.filename, old.content, old.tags, old.summary);
            INSERT INTO documents_fts(rowid, filename, content, tags, summary)
            VALUES (new.id, new.filename, new.content, new.tags, new.summary);
        END;",
    )?;

    init_embedding_schema(&conn)?;

    println!("✅ omniown.db 初始化完成，数据表结构已就绪。\n");
    Ok(())
}

// ---- Embedding Schema ----

pub fn init_embedding_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS document_embeddings (
            -- TODO(Option B): Change PK to (document_id, model_name) for multi-model support
            document_id INTEGER PRIMARY KEY,
            model_name TEXT NOT NULL,
            dim INTEGER NOT NULL,
            vector BLOB NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(document_id) REFERENCES documents(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_document_embeddings_model_name
        ON document_embeddings(model_name);

        CREATE INDEX IF NOT EXISTS idx_documents_embedding_status
        ON documents(embedding_status);",
    )?;
    Ok(())
}

// ---- Embedding 结构体 ----

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct EmbeddingDocument {
    pub id: i64,
    pub filename: String,
    pub stored_path: String,
    pub folder_type: String,
    pub category: String,
    pub content: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DocumentEmbeddingRow {
    pub document_id: i64,
    pub filename: String,
    pub stored_path: String,
    pub folder_type: String,
    pub category: String,
    pub model_name: String,
    pub dim: usize,
    pub vector_blob: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SemanticSearchResult {
    pub document_id: i64,
    pub filename: String,
    pub stored_path: String,
    pub folder_type: String,
    pub category: String,
    pub score: f32,
}

// ---- Embedding CRUD ----

pub fn list_pending_embedding_documents(
    conn: &Connection,
    limit: usize,
) -> rusqlite::Result<Vec<EmbeddingDocument>> {
    let mut stmt = conn.prepare(
        "SELECT id, filename, stored_path, folder_type, category, content
         FROM documents
         WHERE processing_status = 'indexed'
           AND embedding_status = 'pending'
         ORDER BY imported_at ASC
         LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(EmbeddingDocument {
            id: row.get(0)?,
            filename: row.get(1)?,
            stored_path: row.get(2)?,
            folder_type: row.get(3)?,
            category: row.get(4)?,
            content: row.get(5)?,
        })
    })?;

    rows.collect()
}

pub fn upsert_document_embedding(
    conn: &Connection,
    document_id: i64,
    model_name: &str,
    dim: usize,
    vector_blob: &[u8],
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO document_embeddings (document_id, model_name, dim, vector, updated_at)
         VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)
         ON CONFLICT(document_id) DO UPDATE SET
            model_name = excluded.model_name,
            dim = excluded.dim,
            vector = excluded.vector,
            updated_at = CURRENT_TIMESTAMP",
        params![document_id, model_name, dim as i64, vector_blob],
    )?;
    Ok(())
}

pub fn update_embedding_status(
    conn: &Connection,
    document_id: i64,
    status: &str,
) -> rusqlite::Result<()> {
    match status {
        "pending" | "skipped" | "done" | "failed" => {}
        _ => {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "无效的 embedding_status: {}",
                status
            )));
        }
    }

    conn.execute(
        "UPDATE documents SET embedding_status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![status, document_id],
    )?;
    Ok(())
}

#[allow(clippy::type_complexity)]
pub fn list_embeddings_for_search(
    conn: &Connection,
    model_name: Option<&str>,
    folder_type: Option<&str>,
    limit: usize,
) -> rusqlite::Result<Vec<DocumentEmbeddingRow>> {
    let rows: Vec<(i64, String, String, String, String, String, i64, Vec<u8>)> =
        match (model_name, folder_type) {
            (Some(mn), Some(ft)) => {
                let mut stmt = conn.prepare(
                    "SELECT d.id, d.filename, d.stored_path, d.folder_type, d.category,
                            e.model_name, e.dim, e.vector
                     FROM document_embeddings e
                     JOIN documents d ON d.id = e.document_id
                     WHERE d.embedding_status = 'done'
                       AND e.model_name = ?1
                       AND d.folder_type = ?2
                     ORDER BY d.updated_at DESC
                     LIMIT ?3",
                )?;
                stmt.query_map(params![mn, ft, limit as i64], map_embedding_row)?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            }
            (Some(mn), None) => {
                let mut stmt = conn.prepare(
                    "SELECT d.id, d.filename, d.stored_path, d.folder_type, d.category,
                            e.model_name, e.dim, e.vector
                     FROM document_embeddings e
                     JOIN documents d ON d.id = e.document_id
                     WHERE d.embedding_status = 'done'
                       AND e.model_name = ?1
                     ORDER BY d.updated_at DESC
                     LIMIT ?2",
                )?;
                stmt.query_map(params![mn, limit as i64], map_embedding_row)?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            }
            (None, Some(ft)) => {
                let mut stmt = conn.prepare(
                    "SELECT d.id, d.filename, d.stored_path, d.folder_type, d.category,
                            e.model_name, e.dim, e.vector
                     FROM document_embeddings e
                     JOIN documents d ON d.id = e.document_id
                     WHERE d.embedding_status = 'done'
                       AND d.folder_type = ?1
                     ORDER BY d.updated_at DESC
                     LIMIT ?2",
                )?;
                stmt.query_map(params![ft, limit as i64], map_embedding_row)?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            }
            (None, None) => {
                let mut stmt = conn.prepare(
                    "SELECT d.id, d.filename, d.stored_path, d.folder_type, d.category,
                            e.model_name, e.dim, e.vector
                     FROM document_embeddings e
                     JOIN documents d ON d.id = e.document_id
                     WHERE d.embedding_status = 'done'
                     ORDER BY d.updated_at DESC
                     LIMIT ?1",
                )?;
                stmt.query_map(params![limit as i64], map_embedding_row)?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            }
        };

    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        let blob: Vec<u8> = row.7;
        results.push(DocumentEmbeddingRow {
            document_id: row.0,
            filename: row.1,
            stored_path: row.2,
            folder_type: row.3,
            category: row.4,
            model_name: row.5,
            dim: row.6 as usize,
            vector_blob: blob,
        });
    }

    Ok(results)
}

#[allow(clippy::type_complexity)]
fn map_embedding_row(
    row: &rusqlite::Row,
) -> rusqlite::Result<(i64, String, String, String, String, String, i64, Vec<u8>)> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
    ))
}

// ---- FTS 全文检索 ----

#[allow(dead_code)]
pub fn rebuild_fts_index(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO documents_fts(documents_fts) VALUES('rebuild')",
        [],
    )?;
    println!("🔍 FTS 索引重建完成");
    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: i64,
    pub filename: String,
    pub stored_path: String,
    pub folder_type: String,
    pub category: String,
    pub snippet: Option<String>,
    pub rank: f64,
    pub updated_at: String,
}

pub fn search_documents(
    conn: &Connection,
    query: &str,
    limit: i64,
) -> rusqlite::Result<Vec<SearchResult>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let limit = if limit <= 0 { 20 } else { limit };

    let sql = "SELECT d.id, d.filename, d.stored_path, d.folder_type, d.category,
        snippet(documents_fts, 1, '[', ']', '...', 12) AS snippet,
        bm25(documents_fts) AS rank, d.updated_at
    FROM documents_fts
    JOIN documents d ON d.id = documents_fts.rowid
    WHERE documents_fts MATCH ?1
    ORDER BY rank
    LIMIT ?2";

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![query, limit], |row| {
        Ok(SearchResult {
            id: row.get(0)?,
            filename: row.get(1)?,
            stored_path: row.get(2)?,
            folder_type: row.get(3)?,
            category: row.get(4)?,
            snippet: row.get(5)?,
            rank: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    rows.collect()
}

#[allow(dead_code)]
pub fn search_documents_filtered(
    conn: &Connection,
    query: &str,
    folder_type: Option<&str>,
    category: Option<&str>,
    limit: i64,
) -> rusqlite::Result<Vec<SearchResult>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let limit = if limit <= 0 { 20 } else { limit };

    let mut sql = String::from(
        "SELECT d.id, d.filename, d.stored_path, d.folder_type, d.category,
            snippet(documents_fts, 1, '[', ']', '...', 12) AS snippet,
            bm25(documents_fts) AS rank, d.updated_at
        FROM documents_fts
        JOIN documents d ON d.id = documents_fts.rowid
        WHERE documents_fts MATCH ?1",
    );

    if folder_type.is_some() {
        sql.push_str(" AND d.folder_type = ?2");
    }
    if category.is_some() {
        let n = if folder_type.is_some() { 3 } else { 2 };
        sql.push_str(&format!(" AND d.category = ?{}", n));
    }

    let last_param = if folder_type.is_some() && category.is_some() {
        4
    } else if folder_type.is_some() || category.is_some() {
        3
    } else {
        2
    };
    sql.push_str(&format!(" ORDER BY rank LIMIT ?{}", last_param));

    let mut stmt = conn.prepare(&sql)?;

    let mapper = |row: &rusqlite::Row| {
        Ok(SearchResult {
            id: row.get(0)?,
            filename: row.get(1)?,
            stored_path: row.get(2)?,
            folder_type: row.get(3)?,
            category: row.get(4)?,
            snippet: row.get(5)?,
            rank: row.get(6)?,
            updated_at: row.get(7)?,
        })
    };

    match (folder_type, category) {
        (Some(ft), Some(cat)) => {
            let mut stmt = stmt;
            let rows = stmt.query_map(params![query, ft, cat, limit], mapper)?;
            rows.collect()
        }
        (Some(ft), None) => {
            let rows = stmt.query_map(params![query, ft, limit], mapper)?;
            rows.collect()
        }
        (None, Some(cat)) => {
            let rows = stmt.query_map(params![query, cat, limit], mapper)?;
            rows.collect()
        }
        (None, None) => {
            let rows = stmt.query_map(params![query, limit], mapper)?;
            rows.collect()
        }
    }
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

        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
                filename, content, tags, summary,
                content='documents', content_rowid='id'
            )",
            [],
        )
        .unwrap();

        conn.execute_batch(
            "CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
                INSERT INTO documents_fts(rowid, filename, content, tags, summary)
                VALUES (new.id, new.filename, new.content, new.tags, new.summary);
            END;
            CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
                INSERT INTO documents_fts(documents_fts, rowid, filename, content, tags, summary)
                VALUES('delete', old.id, old.filename, old.content, old.tags, old.summary);
            END;
            CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE ON documents BEGIN
                INSERT INTO documents_fts(documents_fts, rowid, filename, content, tags, summary)
                VALUES('delete', old.id, old.filename, old.content, old.tags, old.summary);
                INSERT INTO documents_fts(rowid, filename, content, tags, summary)
                VALUES (new.id, new.filename, new.content, new.tags, new.summary);
            END;",
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

    #[test]
    fn count_empty_database() {
        let conn = setup_db();
        assert_eq!(count_documents(&conn).unwrap(), 0);
        assert_eq!(count_by_folder_type(&conn, "public").unwrap(), 0);
    }

    #[test]
    fn count_documents_after_insert() {
        let conn = setup_db();
        upsert_document(&conn, &make_input("a.md", "lib/pub/notes/a.md", "A")).unwrap();
        upsert_document(&conn, &make_input("b.md", "lib/pub/notes/b.md", "B")).unwrap();
        assert_eq!(count_documents(&conn).unwrap(), 2);
    }

    #[test]
    fn count_by_folder_type_filters() {
        let conn = setup_db();
        upsert_document(&conn, &make_input("pub.md", "lib/pub/pub.md", "x")).unwrap();

        let priv_input = NewDocument {
            filename: "priv.md",
            stored_path: "lib/priv/priv.md",
            folder_type: "private",
            category: "finance",
            domain: "finance",
            ..make_input("priv.md", "lib/priv/priv.md", "x")
        };
        upsert_document(&conn, &priv_input).unwrap();

        assert_eq!(count_by_folder_type(&conn, "public").unwrap(), 1);
        assert_eq!(count_by_folder_type(&conn, "private").unwrap(), 1);
    }

    #[test]
    fn count_by_processing_status_works() {
        let conn = setup_db();
        upsert_document(&conn, &make_input("ok.md", "lib/ok.md", "ok")).unwrap();

        let failed_input = NewDocument {
            processing_status: "failed",
            ..make_input("fail.md", "lib/fail.md", "bad")
        };
        upsert_document(&conn, &failed_input).unwrap();

        assert_eq!(count_by_processing_status(&conn, "indexed").unwrap(), 1);
        assert_eq!(count_by_processing_status(&conn, "failed").unwrap(), 1);
    }

    #[test]
    fn search_finds_by_filename() {
        let conn = setup_db();
        let input = make_input(
            "rust-learning.md",
            "library/public/notes/rust-learning.md",
            "普通笔记内容",
        );
        upsert_document(&conn, &input).unwrap();

        let results = search_documents(&conn, "rust", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].filename, "rust-learning.md");
    }

    #[test]
    fn search_finds_by_content() {
        let conn = setup_db();
        let input = make_input(
            "note.md",
            "library/public/notes/note.md",
            "这里记录了 notify 文件监听系统",
        );
        upsert_document(&conn, &input).unwrap();

        let results = search_documents(&conn, "notify", 10).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn search_empty_returns_empty() {
        let conn = setup_db();
        let results = search_documents(&conn, "", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_nonexistent_returns_empty() {
        let conn = setup_db();
        upsert_document(&conn, &make_input("a.md", "lib/a.md", "hello")).unwrap();
        let results = search_documents(&conn, "nonexistent_xyz", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_updates_with_fts() {
        let conn = setup_db();
        upsert_document(&conn, &make_input("u.md", "lib/u.md", "hello world")).unwrap();
        assert!(!search_documents(&conn, "hello", 10).unwrap().is_empty());

        upsert_document(&conn, &make_input("u.md", "lib/u.md", "goodbye world")).unwrap();
        assert!(search_documents(&conn, "hello", 10).unwrap().is_empty());
        assert!(!search_documents(&conn, "goodbye", 10).unwrap().is_empty());
    }

    #[test]
    fn search_deleted_not_found() {
        let conn = setup_db();
        upsert_document(&conn, &make_input("del.md", "lib/del.md", "delete me")).unwrap();
        assert!(!search_documents(&conn, "delete", 10).unwrap().is_empty());

        delete_document_by_stored_path(&conn, "lib/del.md").unwrap();
        assert!(search_documents(&conn, "delete", 10).unwrap().is_empty());
    }

    #[test]
    fn search_filtered_by_folder_type() {
        let conn = setup_db();
        upsert_document(&conn, &make_input("pub.md", "lib/pub.md", "test keyword")).unwrap();

        let priv_input = NewDocument {
            folder_type: "private",
            category: "finance",
            domain: "finance",
            privacy_score: 0.9,
            risk_level: "medium",
            ..make_input("priv.md", "lib/priv.md", "test keyword")
        };
        upsert_document(&conn, &priv_input).unwrap();

        let all = search_documents_filtered(&conn, "keyword", None, None, 10).unwrap();
        assert_eq!(all.len(), 2);

        let pub_only =
            search_documents_filtered(&conn, "keyword", Some("public"), None, 10).unwrap();
        assert_eq!(pub_only.len(), 1);
        assert_eq!(pub_only[0].folder_type, "public");

        let priv_only =
            search_documents_filtered(&conn, "keyword", Some("private"), None, 10).unwrap();
        assert_eq!(priv_only.len(), 1);
        assert_eq!(priv_only[0].folder_type, "private");
    }

    #[test]
    fn rebuild_fts_index_works() {
        let conn = setup_db();
        upsert_document(&conn, &make_input("r.md", "lib/r.md", "rebuild test")).unwrap();
        rebuild_fts_index(&conn).unwrap();
        let results = search_documents(&conn, "rebuild", 10).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn init_embedding_schema_creates_table() {
        let conn = setup_db();
        init_embedding_schema(&conn).unwrap();
        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='document_embeddings'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(cnt > 0);
    }

    #[test]
    fn upsert_document_embedding_inserts() {
        let conn = setup_db();
        init_embedding_schema(&conn).unwrap();
        let (_, doc) = upsert_document(&conn, &make_input("e.md", "lib/e.md", "hello")).unwrap();

        let blob = vec![0u8; 384 * 4];
        upsert_document_embedding(&conn, doc.id, "mock-384", 384, &blob).unwrap();

        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM document_embeddings WHERE document_id = ?1",
                params![doc.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 1);
    }

    #[test]
    fn update_embedding_status_accepts_valid() {
        let conn = setup_db();
        let (_, doc) =
            upsert_document(&conn, &make_input("s.md", "lib/s.md", "status test")).unwrap();

        update_embedding_status(&conn, doc.id, "done").unwrap();
        update_embedding_status(&conn, doc.id, "failed").unwrap();
        update_embedding_status(&conn, doc.id, "skipped").unwrap();
        update_embedding_status(&conn, doc.id, "pending").unwrap();
    }

    #[test]
    fn update_embedding_status_rejects_invalid() {
        let conn = setup_db();
        let (_, doc) = upsert_document(&conn, &make_input("bad.md", "lib/bad.md", "bad")).unwrap();

        assert!(update_embedding_status(&conn, doc.id, "invalid").is_err());
    }

    #[test]
    fn list_pending_embedding_documents_filters() {
        let conn = setup_db();
        init_embedding_schema(&conn).unwrap();

        let (_, doc1) =
            upsert_document(&conn, &make_input("p1.md", "lib/p1.md", "pending 1")).unwrap();
        update_embedding_status(&conn, doc1.id, "pending").unwrap();

        let (_, doc2) =
            upsert_document(&conn, &make_input("p2.md", "lib/p2.md", "already done")).unwrap();
        update_embedding_status(&conn, doc2.id, "done").unwrap();

        let pending = list_pending_embedding_documents(&conn, 10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, doc1.id);
    }

    #[test]
    fn list_embeddings_for_search_returns_done_only() {
        let conn = setup_db();
        init_embedding_schema(&conn).unwrap();

        let (_, doc1) =
            upsert_document(&conn, &make_input("s1.md", "lib/s1.md", "searchable")).unwrap();
        let blob = vec![0u8; 384 * 4];
        upsert_document_embedding(&conn, doc1.id, "mock-384", 384, &blob).unwrap();
        update_embedding_status(&conn, doc1.id, "done").unwrap();

        let (_, doc2) =
            upsert_document(&conn, &make_input("s2.md", "lib/s2.md", "pending")).unwrap();
        upsert_document_embedding(&conn, doc2.id, "mock-384", 384, &blob).unwrap();
        update_embedding_status(&conn, doc2.id, "pending").unwrap();

        let results = list_embeddings_for_search(&conn, None, None, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, doc1.id);
    }

    #[test]
    fn list_embeddings_for_search_filters_by_model_name() {
        let conn = setup_db();
        init_embedding_schema(&conn).unwrap();

        let (_, doc1) = upsert_document(&conn, &make_input("a.md", "lib/a.md", "text")).unwrap();
        let blob = vec![0u8; 384 * 4];
        upsert_document_embedding(&conn, doc1.id, "mock-a", 384, &blob).unwrap();
        update_embedding_status(&conn, doc1.id, "done").unwrap();

        let (_, doc2) = upsert_document(&conn, &make_input("b.md", "lib/b.md", "text")).unwrap();
        upsert_document_embedding(&conn, doc2.id, "mock-b", 384, &blob).unwrap();
        update_embedding_status(&conn, doc2.id, "done").unwrap();

        // No filter: both returned
        let all = list_embeddings_for_search(&conn, None, None, 10).unwrap();
        assert_eq!(all.len(), 2);

        // Filter by model_name: only matching returned
        let filtered = list_embeddings_for_search(&conn, Some("mock-a"), None, 10).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].model_name, "mock-a");
    }

    #[test]
    fn list_embeddings_for_search_filters_by_model_and_folder() {
        let conn = setup_db();
        init_embedding_schema(&conn).unwrap();

        let mut pub_input = make_input("pub.md", "lib/pub/pub.md", "text");
        pub_input.folder_type = "public";
        let (_, doc1) = upsert_document(&conn, &pub_input).unwrap();
        let blob = vec![0u8; 384 * 4];
        upsert_document_embedding(&conn, doc1.id, "mock-a", 384, &blob).unwrap();
        update_embedding_status(&conn, doc1.id, "done").unwrap();

        let mut priv_input = make_input("priv.md", "lib/priv/priv.md", "text");
        priv_input.folder_type = "private";
        let (_, doc2) = upsert_document(&conn, &priv_input).unwrap();
        upsert_document_embedding(&conn, doc2.id, "mock-a", 384, &blob).unwrap();
        update_embedding_status(&conn, doc2.id, "done").unwrap();

        let results =
            list_embeddings_for_search(&conn, Some("mock-a"), Some("public"), 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].model_name, "mock-a");
        assert_eq!(results[0].folder_type, "public");
    }

    #[test]
    fn count_embeddings_works() {
        let conn = setup_db();
        init_embedding_schema(&conn).unwrap();
        assert_eq!(count_embeddings(&conn).unwrap(), 0);

        let (_, doc) = upsert_document(&conn, &make_input("e.md", "lib/e.md", "text")).unwrap();
        let blob = vec![0u8; 384 * 4];
        upsert_document_embedding(&conn, doc.id, "mock", 384, &blob).unwrap();
        assert_eq!(count_embeddings(&conn).unwrap(), 1);
    }

    #[test]
    fn count_pending_embeddings_works() {
        let conn = setup_db();
        assert_eq!(count_pending_embeddings(&conn).unwrap(), 0);

        let input = make_input("p.md", "lib/p.md", "pending embedding");
        upsert_document(&conn, &input).unwrap();
        assert_eq!(count_pending_embeddings(&conn).unwrap(), 1);

        let (_, doc) = upsert_document(&conn, &input).unwrap();
        update_embedding_status(&conn, doc.id, "done").unwrap();
        assert_eq!(count_pending_embeddings(&conn).unwrap(), 0);
    }
}
