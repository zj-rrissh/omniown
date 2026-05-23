use crate::config::AppConfig;
use crate::db;
use crate::embedding::create_embedding_provider;
use crate::fs_layout::AppPaths;
use crate::migration;

pub fn run_doctor(config: &AppConfig, app_paths: &AppPaths) {
    println!("OmniOwn System Doctor");
    println!("{}\n", "=".repeat(21));

    // [1/5] Paths
    println!("[1/5] Directory Paths");
    let dirs: &[(&str, &std::path::Path)] = &[
        ("root", &app_paths.root),
        ("inbox", &app_paths.inbox),
        ("library", &app_paths.library),
        ("public", &app_paths.public),
        ("private", &app_paths.private),
        ("index", &app_paths.index),
        ("cache", &app_paths.cache),
        ("logs", &app_paths.logs),
        ("quarantine", &app_paths.quarantine),
        ("trash", &app_paths.trash),
        ("config", &app_paths.config),
    ];
    for (name, path) in dirs {
        let status = if path.exists() { "OK" } else { "MISSING" };
        println!("  [{status}] {name}: {}", path.display());
    }
    println!();

    // [2/5] Database
    println!("[2/5] Database");
    match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(conn) => {
            println!("  [OK] can open database: {}", app_paths.db_path.display());

            // Schema version
            match migration::ensure_migration_table(&conn) {
                Ok(()) => println!("  [OK] schema_migrations table exists"),
                Err(_) => println!("  [WARN] schema_migrations table not found"),
            }

            let schema_version = migration::current_version(&conn).unwrap_or(0);
            let pending = migration::pending_count(&conn).unwrap_or(-1);

            if pending == 0 {
                println!("  [OK] current schema version: {schema_version}");
                println!("  [OK] pending migrations: {pending}");
            } else {
                println!("  [WARN] current schema version: {schema_version}");
                println!("  [WARN] pending migrations: {pending} — 请运行 `cargo run -- migrate`");
            }

            // 检查 document_embeddings 主键
            let pk_ok = conn
                .prepare("PRAGMA table_info(document_embeddings)")
                .ok()
                .map(|mut stmt| {
                    let pks: Vec<(String, i64)> = stmt
                        .query_map([], |row| {
                            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?))
                        })
                        .ok()
                        .into_iter()
                        .flatten()
                        .filter_map(|r| r.ok())
                        .filter(|(_, pk)| *pk > 0)
                        .collect();
                    pks.len() == 2
                        && pks.contains(&("document_id".to_string(), 1))
                        && pks.contains(&("model_name".to_string(), 2))
                })
                .unwrap_or(false);

            if pk_ok {
                println!("  [OK] document_embeddings primary key: (document_id, model_name)");
            } else {
                println!("  [WARN] document_embeddings is using legacy single-column primary key");
            }

            let total = db::count_documents(&conn).unwrap_or(0);
            let embeddings = db::count_embeddings(&conn).unwrap_or(0);
            println!("  documents: {total}");
            println!("  embeddings: {embeddings}");
        }
        Err(e) => {
            println!("  [FAIL] cannot open database: {e}");
        }
    }
    println!();

    // [3/5] Embedding Provider
    println!("[3/5] Embedding Provider");
    let dim = config.embedding.dim;
    match create_embedding_provider(config.embedding.provider, dim) {
        Ok(provider) => {
            let functional = provider.embed("ping").is_ok();
            let status = if functional {
                "available"
            } else {
                "experimental"
            };
            println!(
                "  [{status}] provider: {}",
                config.embedding.provider.as_str()
            );
            println!("  model: {}", provider.model_name());
            println!("  dim: {}", provider.dimension());
            println!(
                "  functional: {}",
                if functional { "yes" } else { "no (stub)" }
            );
        }
        Err(e) => {
            println!("  [FAIL] cannot create provider: {e}");
        }
    }
    println!();

    // [4/5] Worker
    println!("[4/5] Worker Configuration");
    println!("  enabled: {}", config.worker.enabled);
    println!("  idle_interval_ms: {}", config.worker.idle_interval_ms);
    println!("  batch_size: {}", config.worker.batch_size);
    println!("  max_docs_per_cycle: {}", config.worker.max_docs_per_cycle);
    println!();

    // [5/5] Search
    println!("[5/5] Search Configuration");
    println!("  default_limit: {}", config.search.default_limit);
    println!(
        "  fts: {}",
        if config.search.fts_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  semantic: {}",
        if config.search.semantic_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!();

    println!("Doctor check complete.");
}

pub fn print_status(config: &AppConfig, app_paths: &AppPaths) {
    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("无法打开数据库: {e}");
            return;
        }
    };

    let total = db::count_documents(&conn).unwrap_or(0);
    let public = db::count_by_folder_type(&conn, "public").unwrap_or(0);
    let private = db::count_by_folder_type(&conn, "private").unwrap_or(0);
    let indexed = db::count_by_processing_status(&conn, "indexed").unwrap_or(0);
    let failed = db::count_by_processing_status(&conn, "failed").unwrap_or(0);
    let embedding_count = db::count_embeddings(&conn).unwrap_or(0);
    let pending_embeddings = db::count_pending_embeddings(&conn).unwrap_or(0);
    let schema_version = migration::current_version(&conn).unwrap_or(0);
    let pending_migrations = migration::pending_count(&conn).unwrap_or(-1);

    // 当前 provider 的 model-aware 计数
    let current_model = match crate::embedding::create_embedding_provider(
        config.embedding.provider,
        config.embedding.dim,
    ) {
        Ok(p) => {
            let mn = p.model_name().to_string();
            let model_embeddings = db::count_embeddings_for_model(&conn, &mn).unwrap_or(0);
            let model_pending = db::count_pending_embeddings_for_model(&conn, &mn).unwrap_or(0);
            Some((mn, model_embeddings, model_pending))
        }
        Err(_) => None,
    };

    println!("\nOmniOwn Status\n");
    println!("Database: {}", app_paths.db_path.display());
    println!("Root: {}", app_paths.root.display());
    println!();
    println!("Schema:");
    println!("  current_version: {schema_version}");
    println!("  pending_migrations: {pending_migrations}");
    println!();
    println!("Documents:");
    println!("  total:    {total}");
    println!("  public:   {public}");
    println!("  private:  {private}");
    println!("  indexed:  {indexed}");
    println!("  failed:   {failed}");
    println!();
    println!("Provider:  {}", config.embedding.provider.as_str());
    if let Some((model, model_emb, model_pend)) = &current_model {
        println!("Embeddings:");
        println!("  total:    {embedding_count}");
        println!("  current_model: {model}");
        println!("  current_model_embeddings: {model_emb}");
        println!("  pending_for_current_model: {model_pend}");
    } else {
        println!("Embeddings:");
        println!("  total:    {embedding_count}");
        println!("  pending:  {pending_embeddings}");
    }
    println!();
    println!(
        "Worker:    {}",
        if config.worker.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::db;
    use crate::migration;
    use std::fs;

    #[test]
    fn doctor_runs_with_default_config() {
        let root = std::env::temp_dir().join(format!("omniown_doctor_test_{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let app_paths = AppPaths::new(&root);
        app_paths.init_directories().unwrap();

        let config = AppConfig::default();
        run_doctor(&config, &app_paths);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn status_shows_counts() {
        let root = std::env::temp_dir().join(format!("omniown_status_test_{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let app_paths = AppPaths::new(&root);
        app_paths.init_directories().unwrap();
        db::init_database(&app_paths.db_path).unwrap();

        let config = AppConfig::default();

        let conn = rusqlite::Connection::open(&app_paths.db_path).unwrap();
        conn.execute(
            "INSERT INTO documents (filename, stored_path, file_hash, folder_type, category, processing_status)
             VALUES ('test.md', 'lib/test.md', 'abc123', 'public', 'notes', 'indexed')",
            [],
        )
        .unwrap();

        print_status(&config, &app_paths);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn doctor_reports_schema_version() {
        let root =
            std::env::temp_dir().join(format!("omniown_doctor_schema_{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let app_paths = AppPaths::new(&root);
        app_paths.init_directories().unwrap();
        db::init_database(&app_paths.db_path).unwrap();

        let conn = rusqlite::Connection::open(&app_paths.db_path).unwrap();
        let version = migration::current_version(&conn).unwrap_or(0);
        assert_eq!(version, 5, "doctor 应报告 schema version 为 5");

        let pending = migration::pending_count(&conn).unwrap_or(-1);
        assert_eq!(pending, 0, "doctor 应报告 pending migrations 为 0");

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn status_reports_schema_version() {
        let root =
            std::env::temp_dir().join(format!("omniown_status_schema_{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let app_paths = AppPaths::new(&root);
        app_paths.init_directories().unwrap();
        db::init_database(&app_paths.db_path).unwrap();

        let config = AppConfig::default();
        print_status(&config, &app_paths);

        fs::remove_dir_all(&root).ok();
    }
}
