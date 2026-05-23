use crate::db;
use crate::fs_layout::AppPaths;
use crate::processor::{self, ExistingFileDecision};
use crate::storage;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEMP_PROJECT_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn make_temp_project() -> (AppPaths, PathBuf) {
    let counter = TEMP_PROJECT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "omniown_batch_test_{}_{}",
        std::process::id(),
        counter
    ));
    fs::create_dir_all(&root).unwrap();

    let app_paths = AppPaths::new(&root);
    app_paths.init_directories().unwrap();
    db::init_database(&app_paths.db_path).unwrap();

    (app_paths, root)
}

fn generate_file(inbox: &PathBuf, name: &str, content: &str) -> PathBuf {
    let path = inbox.join(name);
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn bootstrap_prefers_root_toml_over_config_dir() {
    let id = std::process::id();
    let root = std::env::temp_dir().join(format!("omniown_bootstrap_root_{}", id));
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    fs::write(
        root.join("omniown.toml"),
        r#"[paths]
database = "root.db"
"#,
    )
    .unwrap();
    fs::write(
        config_dir.join("omniown.toml"),
        r#"[paths]
database = "config.db"
"#,
    )
    .unwrap();

    unsafe { std::env::set_var("OMNIOWN_ROOT", root.to_string_lossy().as_ref()) };
    let (config, _) = crate::bootstrap();
    unsafe { std::env::remove_var("OMNIOWN_ROOT") };

    assert_eq!(config.paths.database, root.join("root.db"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn batch_import_100_files() {
    let (app_paths, root) = make_temp_project();

    for i in 1..=50 {
        let name = format!("note_{:03}.md", i);
        generate_file(
            &app_paths.inbox,
            &name,
            &format!("# Note {}\n\n这是一篇普通学习笔记\n\n内容序号: {}", i, i),
        );
    }

    for i in 1..=20 {
        let name = format!("code_{:03}.rs", i);
        generate_file(
            &app_paths.inbox,
            &name,
            &format!(
                "// Code file {}\nfn main() {{\n    println!(\"hello {}\");\n}}\n",
                i, i
            ),
        );
    }

    for i in 1..=15 {
        let name = format!("invoice_{:03}.md", i);
        generate_file(&app_paths.inbox, &name, &format!("# 报销单 {}", i));
    }

    for i in 1..=10 {
        let name = format!("identity_{:03}.txt", i);
        generate_file(
            &app_paths.inbox,
            &name,
            &format!("api_key = \"sk-test-{}\"\nsecret = \"xyz{}\"\n", i, i),
        );
    }

    for i in 1..=5 {
        let name = format!("journal_{:03}.md", i);
        generate_file(
            &app_paths.inbox,
            &name,
            &format!("# 今日日记 {}\n\n今天心情很好，很开心\n", i),
        );
    }

    let mut entries: Vec<_> = fs::read_dir(&app_paths.inbox)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();

    assert_eq!(entries.len(), 100, "应该有 100 个文件在 inbox 中");

    for entry in &entries {
        processor::process_file(entry, &app_paths).ok();
    }

    let remaining: Vec<_> = fs::read_dir(&app_paths.inbox)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    assert!(
        remaining.is_empty(),
        "inbox 应为空，但剩下 {} 个文件",
        remaining.len()
    );

    let conn = rusqlite::Connection::open(&app_paths.db_path).unwrap();

    let library_files = fs::read_dir(&app_paths.public).unwrap().count()
        + fs::read_dir(&app_paths.private).unwrap().count();
    assert_eq!(
        library_files, 100,
        "应有 100 个文件落在测试项目的 library 中，实际 {}",
        library_files
    );

    let docs = db::list_documents_meta(&conn).unwrap();
    assert!(
        docs.iter()
            .all(|doc| doc.stored_path.starts_with("library/")),
        "数据库 stored_path 应保持为 root-relative library 路径"
    );
    assert!(
        docs.iter().all(|doc| {
            doc.stored_path
                .rsplit('/')
                .next()
                .is_some_and(|name| !storage::is_old_library_filename(name))
        }),
        "新导入路径不应包含日期/hash 前缀"
    );

    let total = db::count_documents(&conn).unwrap();
    assert_eq!(total, 100, "应有 100 条数据库记录，实际 {}", total);

    let public = db::count_by_folder_type(&conn, "public").unwrap();
    assert_eq!(public, 70, "应为 70 个 public 文档，实际 {}", public);

    let private = db::count_by_folder_type(&conn, "private").unwrap();
    assert_eq!(private, 30, "应为 30 个 private 文档，实际 {}", private);

    let notes = db::count_by_category(&conn, "notes").unwrap();
    assert_eq!(notes, 50, "应为 50 个 notes，实际 {}", notes);

    let code = db::count_by_category(&conn, "code").unwrap();
    assert_eq!(code, 20, "应为 20 个 code，实际 {}", code);

    let finance = db::count_by_category(&conn, "finance").unwrap();
    assert_eq!(finance, 15, "应为 15 个 finance，实际 {}", finance);

    let identity = db::count_by_category(&conn, "identity").unwrap();
    assert_eq!(identity, 10, "应为 10 个 identity，实际 {}", identity);

    let journal = db::count_by_category(&conn, "journal").unwrap();
    assert_eq!(journal, 5, "应为 5 个 journal，实际 {}", journal);

    let failed = db::count_by_processing_status(&conn, "failed").unwrap();
    assert_eq!(failed, 0, "应有 0 个失败，实际 {}", failed);

    let indexed = db::count_by_processing_status(&conn, "indexed").unwrap();
    assert_eq!(indexed, 100, "应有 100 个 indexed，实际 {}", indexed);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn import_conflict_cancel_keeps_inbox_file_and_existing_library_file() {
    let (app_paths, root) = make_temp_project();
    let inbox_path = generate_file(&app_paths.inbox, "same.md", "# New\n");
    let library_path = app_paths.public.join("same.md");
    generate_file(&app_paths.public, "same.md", "# Existing\n");

    processor::process_file_with_conflict_decision(
        &inbox_path,
        &app_paths,
        Some(ExistingFileDecision::Cancel),
    )
    .unwrap();

    assert!(inbox_path.exists());
    assert_eq!(fs::read_to_string(&library_path).unwrap(), "# Existing\n");

    let conn = rusqlite::Connection::open(&app_paths.db_path).unwrap();
    assert_eq!(db::count_documents(&conn).unwrap(), 0);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn import_conflict_overwrite_replaces_library_file_and_updates_db() {
    let (app_paths, root) = make_temp_project();
    let inbox_path = generate_file(&app_paths.inbox, "same.md", "# New\n");
    let library_path = app_paths.public.join("same.md");
    generate_file(&app_paths.public, "same.md", "# Existing\n");

    processor::process_file_with_conflict_decision(
        &inbox_path,
        &app_paths,
        Some(ExistingFileDecision::Overwrite),
    )
    .unwrap();

    assert!(!inbox_path.exists());
    assert_eq!(fs::read_to_string(&library_path).unwrap(), "# New\n");

    let conn = rusqlite::Connection::open(&app_paths.db_path).unwrap();
    let docs = db::list_documents_meta(&conn).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].stored_path, "library/public/same.md");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn html_import_extracts_searchable_text() {
    let (app_paths, root) = make_temp_project();

    let path = generate_file(
        &app_paths.inbox,
        "page.html",
        "<html><body><h1>AlphaBeta</h1><script>ShouldNotIndex</script></body></html>",
    );

    processor::process_file(&path, &app_paths).unwrap();

    let conn = rusqlite::Connection::open(&app_paths.db_path).unwrap();
    let docs = db::list_documents_meta(&conn).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].category, "docs");
    assert_eq!(docs[0].doc_type, "html");

    let results = db::search_documents(&conn, "AlphaBeta", 10).unwrap();
    assert_eq!(results.len(), 1);

    let script_results = db::search_documents(&conn, "ShouldNotIndex", 10).unwrap();
    assert!(script_results.is_empty());

    fs::remove_dir_all(&root).ok();
}
