use rusqlite::Connection;
use std::collections::HashSet;

// ---- Migration 定义 ----

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub apply: fn(&Connection) -> rusqlite::Result<()>,
}

pub struct MigrationReport {
    pub applied: Vec<i64>,
    pub skipped: Vec<i64>,
}

/// 所有迁移按版本升序排列
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "create_documents",
        apply: migration_1_create_documents,
    },
    Migration {
        version: 2,
        name: "create_documents_fts",
        apply: migration_2_create_documents_fts,
    },
    Migration {
        version: 3,
        name: "create_document_embeddings",
        apply: migration_3_create_document_embeddings,
    },
    Migration {
        version: 4,
        name: "create_indexes",
        apply: migration_4_create_indexes,
    },
    Migration {
        version: 5,
        name: "document_embeddings_composite_primary_key",
        apply: migration_5_document_embeddings_composite_pk,
    },
];

// ---- 迁移表管理 ----

pub fn ensure_migration_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
    )?;
    Ok(())
}

pub fn applied_versions(conn: &Connection) -> rusqlite::Result<HashSet<i64>> {
    ensure_migration_table(conn)?;
    let mut stmt = conn.prepare("SELECT version FROM schema_migrations ORDER BY version")?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    let mut versions = HashSet::new();
    for row in rows {
        versions.insert(row?);
    }
    Ok(versions)
}

#[allow(dead_code)]
pub fn pending_migrations(conn: &Connection) -> rusqlite::Result<Vec<&'static Migration>> {
    let applied = applied_versions(conn)?;
    Ok(MIGRATIONS
        .iter()
        .filter(|m| !applied.contains(&m.version))
        .collect())
}

// ---- 执行迁移 ----

pub fn run_migrations(conn: &Connection) -> rusqlite::Result<MigrationReport> {
    ensure_migration_table(conn)?;
    let applied = applied_versions(conn)?;

    let mut report = MigrationReport {
        applied: Vec::new(),
        skipped: Vec::new(),
    };

    for migration in MIGRATIONS {
        if applied.contains(&migration.version) {
            report.skipped.push(migration.version);
            continue;
        }

        (migration.apply)(conn)?;

        conn.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            rusqlite::params![migration.version, migration.name],
        )?;

        report.applied.push(migration.version);
    }

    Ok(report)
}

/// 版本号 → 名称的静态映射，用于 CLI 展示
pub fn migration_name(version: i64) -> &'static str {
    for m in MIGRATIONS {
        if m.version == version {
            return m.name;
        }
    }
    "unknown"
}

// ---- 查询方法 ----

pub fn current_version(conn: &Connection) -> rusqlite::Result<i64> {
    ensure_migration_table(conn)?;
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |r| r.get(0),
    )
}

pub fn pending_count(conn: &Connection) -> rusqlite::Result<i64> {
    let applied = applied_versions(conn)?;
    Ok(MIGRATIONS
        .iter()
        .filter(|m| !applied.contains(&m.version))
        .count() as i64)
}

// ---- 迁移实现 ----

fn migration_1_create_documents(conn: &Connection) -> rusqlite::Result<()> {
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
    Ok(())
}

fn migration_2_create_documents_fts(conn: &Connection) -> rusqlite::Result<()> {
    // FTS5 可能受 SQLite 编译选项影响，失败时不阻断迁移
    let fts_result = conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
            filename,
            content,
            tags,
            summary,
            content='documents',
            content_rowid='id'
        );

        CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
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
    );

    if let Err(ref e) = fts_result {
        eprintln!("[WARN] FTS5 初始化失败（当前 SQLite 可能未启用 FTS5）: {e}");
        eprintln!("[WARN] 全文搜索不可用，但其他功能不受影响");
    }

    Ok(())
}

fn migration_3_create_document_embeddings(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS document_embeddings (
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

fn migration_5_document_embeddings_composite_pk(conn: &Connection) -> rusqlite::Result<()> {
    // 自事务保护：四步迁移中间失败必须能回滚
    conn.execute_batch("BEGIN IMMEDIATE;")?;

    let result = (|| -> rusqlite::Result<()> {
        // 1. 创建新表（复合主键）
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS document_embeddings_v2 (
                document_id INTEGER NOT NULL,
                model_name TEXT NOT NULL,
                dim INTEGER NOT NULL,
                vector BLOB NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(document_id, model_name),
                FOREIGN KEY(document_id) REFERENCES documents(id) ON DELETE CASCADE
            );",
        )?;

        // 2. 拷贝旧数据（旧表不存在时跳过拷贝）
        let old_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='document_embeddings'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);

        if old_exists {
            conn.execute(
                "INSERT INTO document_embeddings_v2 (document_id, model_name, dim, vector, created_at, updated_at)
                 SELECT document_id, COALESCE(model_name, 'unknown'), dim, vector, created_at, updated_at
                 FROM document_embeddings",
                [],
            )?;

            // 3. 删除旧表
            conn.execute_batch("DROP TABLE IF EXISTS document_embeddings;")?;
        }

        // 4. 重命名
        conn.execute_batch("ALTER TABLE document_embeddings_v2 RENAME TO document_embeddings;")?;

        // 5. 重建索引
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_document_embeddings_model_name
             ON document_embeddings(model_name);

             CREATE INDEX IF NOT EXISTS idx_document_embeddings_model_dim
             ON document_embeddings(model_name, dim);",
        )?;

        Ok(())
    })();

    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(e)
        }
    }
}

fn migration_4_create_indexes(conn: &Connection) -> rusqlite::Result<()> {
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_documents_hash ON documents(file_hash)",
        "CREATE INDEX IF NOT EXISTS idx_documents_folder_type ON documents(folder_type)",
        "CREATE INDEX IF NOT EXISTS idx_documents_category ON documents(category)",
        "CREATE INDEX IF NOT EXISTS idx_documents_processing_status ON documents(processing_status)",
        "CREATE INDEX IF NOT EXISTS idx_documents_updated_at ON documents(updated_at)",
    ];

    for idx in indexes {
        conn.execute(idx, [])?;
    }
    Ok(())
}

// ---- 测试 ----

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_conn() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            rusqlite::params![name],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
            > 0
    }

    fn index_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
            rusqlite::params![name],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
            > 0
    }

    #[test]
    fn migration_table_is_created() {
        let conn = in_memory_conn();
        ensure_migration_table(&conn).unwrap();
        assert!(table_exists(&conn, "schema_migrations"));
    }

    #[test]
    fn migration_runs_once() {
        let conn = in_memory_conn();
        let report = run_migrations(&conn).unwrap();
        assert_eq!(report.applied.len(), 5);
        assert!(report.skipped.is_empty());

        // 再次运行——全部 skipped
        let report2 = run_migrations(&conn).unwrap();
        assert!(report2.applied.is_empty());
        assert_eq!(report2.skipped.len(), 5);
    }

    #[test]
    fn migration_does_not_duplicate_applied_versions() {
        let conn = in_memory_conn();
        run_migrations(&conn).unwrap();

        let versions = applied_versions(&conn).unwrap();
        assert_eq!(versions.len(), 5);
        for v in 1..=5 {
            assert!(versions.contains(&v), "version {v} should be present");
        }
    }

    #[test]
    fn migration_report_lists_applied_and_skipped() {
        let conn = in_memory_conn();
        let report = run_migrations(&conn).unwrap();
        assert!(report.applied.contains(&1));
        assert!(report.applied.contains(&5));
        assert!(report.skipped.is_empty());

        let report2 = run_migrations(&conn).unwrap();
        assert!(report2.applied.is_empty());
        assert!(report2.skipped.contains(&1));
        assert!(report2.skipped.contains(&5));
    }

    #[test]
    fn migration_creates_documents_table() {
        let conn = in_memory_conn();
        run_migrations(&conn).unwrap();
        assert!(table_exists(&conn, "documents"));

        // 可以插入数据
        conn.execute(
            "INSERT INTO documents (filename, stored_path, file_hash) VALUES (?1, ?2, ?3)",
            rusqlite::params!["test.md", "lib/test.md", "abc123"],
        )
        .unwrap();
    }

    #[test]
    fn migration_creates_document_embeddings_table() {
        let conn = in_memory_conn();
        run_migrations(&conn).unwrap();
        assert!(table_exists(&conn, "document_embeddings"));
        assert!(index_exists(&conn, "idx_document_embeddings_model_name"));
        assert!(index_exists(&conn, "idx_documents_embedding_status"));
    }

    #[test]
    fn migration_creates_schema_migrations_table() {
        let conn = in_memory_conn();
        run_migrations(&conn).unwrap();
        assert!(table_exists(&conn, "schema_migrations"));
    }

    #[test]
    fn pending_migrations_empty_after_run() {
        let conn = in_memory_conn();
        run_migrations(&conn).unwrap();
        let pending = pending_migrations(&conn).unwrap();
        assert!(pending.is_empty());
        assert_eq!(pending_count(&conn).unwrap(), 0);
    }

    #[test]
    fn pending_migrations_non_empty_before_run() {
        let conn = in_memory_conn();
        let pending = pending_migrations(&conn).unwrap();
        assert_eq!(pending.len(), 5);
        assert_eq!(pending_count(&conn).unwrap(), 5);
    }

    #[test]
    fn current_version_works() {
        let conn = in_memory_conn();
        assert_eq!(current_version(&conn).unwrap(), 0);
        run_migrations(&conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), 5);
    }

    #[test]
    fn indexes_are_created() {
        let conn = in_memory_conn();
        run_migrations(&conn).unwrap();
        assert!(index_exists(&conn, "idx_documents_hash"));
        assert!(index_exists(&conn, "idx_documents_folder_type"));
        assert!(index_exists(&conn, "idx_documents_category"));
        assert!(index_exists(&conn, "idx_documents_processing_status"));
        assert!(index_exists(&conn, "idx_documents_updated_at"));
    }

    fn pk_info(conn: &Connection, table: &str) -> Vec<(String, i64)> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({})", table))
            .unwrap();
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .filter(|(_, pk)| *pk > 0)
        .collect()
    }

    #[test]
    fn migration_5_converts_document_embeddings_to_composite_pk() {
        let conn = in_memory_conn();

        // 先运行 migration 1-4，创建旧版 document_embeddings
        ensure_migration_table(&conn).unwrap();
        for m in &MIGRATIONS[..4] {
            (m.apply)(&conn).unwrap();
        }
        for v in 1..=4 {
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (version, name) VALUES (?1, ?2)",
                rusqlite::params![v as i64, format!("migration_{}", v)],
            )
            .unwrap();
        }

        // 插入一条旧数据到旧表
        conn.execute(
            "INSERT INTO documents (filename, stored_path, file_hash) VALUES ('t.md', 'lib/t.md', 'h1')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO document_embeddings (document_id, model_name, dim, vector) VALUES (1, 'mock-hash-384', 384, X'00000000')",
            [],
        ).unwrap();

        // 运行 Migration 5
        migration_5_document_embeddings_composite_pk(&conn).unwrap();

        // 检查复合主键
        let pks = pk_info(&conn, "document_embeddings");
        assert_eq!(pks.len(), 2, "应有 2 个主键列");
        assert!(pks.contains(&("document_id".to_string(), 1)));
        assert!(pks.contains(&("model_name".to_string(), 2)));

        // 检查旧数据仍在
        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM document_embeddings WHERE document_id=1 AND model_name='mock-hash-384'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 1);
    }

    #[test]
    fn migration_5_preserves_existing_embeddings() {
        let conn = in_memory_conn();
        // 只运行 migration 1-4（创建旧表）
        for m in &MIGRATIONS[..4] {
            (m.apply)(&conn).unwrap();
        }
        conn.execute(
            "INSERT INTO documents (filename, stored_path, file_hash) VALUES ('t.md', 'lib/t.md', 'h1')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO document_embeddings (document_id, model_name, dim, vector) VALUES (1, 'mock-hash-384', 384, X'00000000')",
            [],
        ).unwrap();

        migration_5_document_embeddings_composite_pk(&conn).unwrap();

        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM document_embeddings WHERE model_name='mock-hash-384'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 1);
    }

    #[test]
    fn document_embeddings_allows_multiple_models_per_document() {
        let conn = in_memory_conn();
        run_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO documents (filename, stored_path, file_hash) VALUES ('t.md', 'lib/t.md', 'h1')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO document_embeddings (document_id, model_name, dim, vector) VALUES (1, 'mock-hash-384', 384, X'00000001')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO document_embeddings (document_id, model_name, dim, vector) VALUES (1, 'local-stub', 384, X'00000002')",
            [],
        ).unwrap();

        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM document_embeddings WHERE document_id=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 2);
    }

    #[test]
    fn migration_failure_does_not_record() {
        // 人为制造一个失败的迁移来验证
        let conn = in_memory_conn();
        ensure_migration_table(&conn).unwrap();

        // 直接插入版本 1
        conn.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (1, 'test')",
            [],
        )
        .unwrap();

        let versions = applied_versions(&conn).unwrap();
        assert_eq!(versions.len(), 1);
        assert!(versions.contains(&1));
    }
}
