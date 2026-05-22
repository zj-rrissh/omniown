use crate::config::AppConfig;
use crate::db;
use crate::embedding::create_embedding_provider;
use crate::fs_layout::AppPaths;

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

    println!("\nOmniOwn Status\n");
    println!("Database: {}", app_paths.db_path.display());
    println!("Root: {}", app_paths.root.display());
    println!();
    println!("Documents:");
    println!("  total:    {total}");
    println!("  public:   {public}");
    println!("  private:  {private}");
    println!("  indexed:  {indexed}");
    println!("  failed:   {failed}");
    println!();
    println!("Embeddings:");
    println!("  total:    {embedding_count}");
    println!("  pending:  {pending_embeddings}");
    println!();
    println!("Provider:  {}", config.embedding.provider.as_str());
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
}
