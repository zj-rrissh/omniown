mod migration;

mod classifier;
mod config;
mod db;
mod extractor;
mod fs_layout;
mod mcp;
mod processor;
mod storage;


use config::AppConfig;
use fs_layout::AppPaths;
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecursiveMode, Result, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DEBOUNCE_DURATION: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
enum FileTask {
    Upsert(PathBuf),
    Remove(PathBuf),
}

fn is_text_file(path: &Path) -> bool {
    processor::is_supported_file(path)
}

fn handle_file_remove(path: &Path, app_paths: &AppPaths) {
    // If the file still physically exists, this Remove event was likely
    // triggered by an overwrite during import (processor removes old file
    // then immediately recreates it). Skip DB deletion to avoid races.
    if path.exists() {
        return;
    }

    let stored_path = path.to_string_lossy().to_string();

    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\u{26a0}\u{fe0f} 打开数据库失败: {}", e);
            return;
        }
    };

    match db::delete_document_by_stored_path(&conn, &stored_path) {
        Ok(true) => println!("\u{1f5d1}\u{fe0f} 已从数据库移除: {}", stored_path),
        Ok(false) => println!(
            "\u{23ed}\u{fe0f} 数据库中无此路径记录，跳过: {}",
            stored_path
        ),
        Err(e) => eprintln!("\u{26a0}\u{fe0f} 数据库删除失败 [{}]: {}", stored_path, e),
    }
}

async fn enqueue_existing_inbox_files(
    app_paths: &AppPaths,
    tx: &tokio::sync::mpsc::Sender<FileTask>,
) {
    let entries = match std::fs::read_dir(&app_paths.inbox) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!(
                "\u{26a0}\u{fe0f} 扫描 inbox 失败 [{}]: {}",
                app_paths.inbox.display(),
                e
            );
            return;
        }
    };

    let mut count = 0;
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.is_file() && is_text_file(&path) {
            count += 1;
            println!("\u{1f4c4} 已有文件入队: {:?}", path);
            if tx.send(FileTask::Upsert(path)).await.is_err() {
                eprintln!("\u{26a0}\u{fe0f} inbox 启动扫描入队失败: worker channel closed");
                return;
            }
        }
    }

    if count > 0 {
        println!(
            "\u{1f4e5} inbox 启动扫描完成，已入队 {} 个已有文件\n",
            count
        );
    }
}

fn run_migrate(app_paths: &AppPaths) {
    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\u{274c} 无法打开数据库: {}", e);
            return;
        }
    };

    println!("OmniOwn Migration\n");

    match migration::run_migrations(&conn) {
        Ok(report) => {
            println!("Applied:");
            if report.applied.is_empty() {
                println!("  none");
            } else {
                for v in &report.applied {
                    let name = migration::migration_name(*v);
                    println!("  - {} {}", v, name);
                }
            }

            println!("\nSkipped:");
            if report.skipped.is_empty() {
                println!("  none");
            } else {
                for v in &report.skipped {
                    let name = migration::migration_name(*v);
                    println!("  - {} {}", v, name);
                }
            }

            let version = migration::current_version(&conn).unwrap_or(0);
            println!("\nCurrent schema version: {}", version);
        }
        Err(e) => {
            eprintln!("\u{274c} 迁移失败: {}", e);
        }
    }
}

fn bootstrap() -> (AppConfig, AppPaths) {
    let initial_root = std::env::var("OMNIOWN_ROOT").unwrap_or_else(|_| ".".to_string());
    let root = PathBuf::from(&initial_root);
    let config_dir = if root.join("omniown.toml").exists() {
        root.clone()
    } else {
        root.join("config")
    };
    let config = AppConfig::load(&config_dir);
    let app_paths = AppPaths::from_config(&config.paths);
    (config, app_paths)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2 && args[1] == "config-example" {
        config::print_example_config();
        return Ok(());
    }

    let (config, app_paths) = bootstrap();

    if args.len() >= 2 {
        match args[1].as_str() {
            "process" if args.len() >= 3 => {
                let path = Path::new(&args[2]);
                match processor::process_file(path, &app_paths) {
                    Ok(()) => {}
                    Err(e) => eprintln!("处理失败: {e}"),
                }
                return Ok(());
            }
            "migrate" => {
                run_migrate(&app_paths);
                return Ok(());
            }
            "mcp" => {
                if let Err(e) = mcp::run_mcp(&config, &app_paths) {
                    eprintln!("\u{274c} MCP server error: {e:#}");
                }
                return Ok(());
            }
            _ => {
                eprintln!("未知命令: {}", args[1]);
                eprintln!("用法: omniown <command> [args]");
                eprintln!("命令: process, migrate, mcp");
                return Ok(());
            }
        }
    }

    run_sentinel(config, app_paths).await
}

async fn run_sentinel(_config: AppConfig, app_paths: AppPaths) -> Result<()> {
    if let Err(e) = app_paths.init_directories() {
        eprintln!("\u{274c} 目录初始化失败: {}", e);
        return Ok(());
    }
    println!("\u{1f4c1} 目录结构初始化完成");

    if let Err(e) = db::init_database(&app_paths.db_path) {
        eprintln!("\u{274c} 数据库初始化失败: {}", e);
        return Ok(());
    }

    println!("数据库: {}", app_paths.db_path.display());

    println!(
        "\u{1f441}\u{fe0f} AI 哨兵已启动，正在监控: {}\n",
        app_paths.inbox.display()
    );

    let last_modify: Arc<Mutex<HashMap<PathBuf, Instant>>> = Arc::new(Mutex::new(HashMap::new()));

    let (tx, mut rx) = tokio::sync::mpsc::channel::<FileTask>(1000);

    let app_paths_bg = app_paths.clone();
    tokio::spawn(async move {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(4));

        while let Some(task) = rx.recv().await {
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("哨兵 semaphore 未关闭");
            let paths = app_paths_bg.clone();

            tokio::spawn(async move {
                let _permit = permit;

                match task {
                    FileTask::Upsert(path) => {
                        let path_clone = path.clone();
                        let result = tokio::task::spawn_blocking(move || {
                            processor::process_file(&path_clone, &paths)
                        })
                        .await;

                        match result {
                            Ok(Ok(())) => {}
                            Ok(Err(e)) => {
                                eprintln!("\u{26a0}\u{fe0f} 处理文件失败 [{:?}]: {}", path, e)
                            }
                            Err(e) => eprintln!("\u{26a0}\u{fe0f} 阻塞任务失败: {}", e),
                        }
                    }
                    FileTask::Remove(path) => {
                        let paths_clone = paths.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            handle_file_remove(&path, &paths_clone);
                        })
                        .await;
                    }
                }
            });
        }
    });

    let mut watcher = notify::recommended_watcher({
        let last_modify = Arc::clone(&last_modify);
        let tx = tx.clone();

        move |res: Result<Event>| {
            let Ok(event) = res else {
                eprintln!("\u{274c} 监控错误: {:?}", res.err());
                return;
            };

            match event.kind {
                EventKind::Access(_) => {}

                EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                    for path in event.paths {
                        if is_text_file(&path) {
                            println!("\u{1f5d1}\u{fe0f} 文件已移除: {:?}", path);
                            let _ = tx.blocking_send(FileTask::Remove(path));
                        }
                    }
                }

                EventKind::Create(_) | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                    for path in event.paths {
                        if is_text_file(&path) {
                            println!("\u{1f4c4} 新文件入队: {:?}", path);
                            let _ = tx.blocking_send(FileTask::Upsert(path));
                        }
                    }
                }

                _ => {
                    let mut map = last_modify.lock().expect("last_modify mutex 未被 poision");
                    let now = Instant::now();

                    for path in event.paths {
                        if !is_text_file(&path) {
                            continue;
                        }

                        if let Some(last) = map.get(&path)
                            && now.duration_since(*last) < DEBOUNCE_DURATION
                        {
                            continue;
                        }

                        map.insert(path.clone(), now);
                        println!("\u{1f4dd} 修改任务入队: {:?}", path);
                        let _ = tx.blocking_send(FileTask::Upsert(path));
                    }
                }
            }
        }
    })?;

    watcher.watch(&app_paths.inbox, RecursiveMode::NonRecursive)?;
    enqueue_existing_inbox_files(&app_paths, &tx).await;

    // Watch library directories for manual file removals, so DB records
    // stay consistent with the file system.
    let mut lib_watcher = notify::recommended_watcher({
        let tx = tx.clone();
        move |res: Result<Event>| {
            let Ok(event) = res else { return };
            match event.kind {
                EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                    for path in event.paths {
                        if is_text_file(&path) {
                            let _ = tx.blocking_send(FileTask::Remove(path));
                        }
                    }
                }
                _ => {}
            }
        }
    })?;
    lib_watcher.watch(&app_paths.library, RecursiveMode::Recursive)?;

    tokio::signal::ctrl_c().await.ok();
    println!("\u{1f44b} 已退出");

    Ok(())
}

#[cfg(test)]
mod main_tests {
    use super::*;
    use std::path::Path;

    // ---- is_text_file ----

    #[test]
    fn is_text_file_supported_extensions() {
        assert!(is_text_file(Path::new("note.md")), "md");
        assert!(is_text_file(Path::new("page.html")), "html");
        assert!(is_text_file(Path::new("main.rs")), "rs");
        assert!(is_text_file(Path::new("data.json")), "json");
        assert!(is_text_file(Path::new("config.toml")), "toml");
        assert!(is_text_file(Path::new("readme.txt")), "txt");
    }

    #[test]
    fn is_text_file_supported_binary_formats() {
        assert!(is_text_file(Path::new("doc.pdf")), "pdf");
        assert!(is_text_file(Path::new("sheet.xlsx")), "xlsx");
        assert!(is_text_file(Path::new("report.docx")), "docx");
        assert!(is_text_file(Path::new("slides.pptx")), "pptx");
    }

    #[test]
    fn is_text_file_unsupported_extensions() {
        assert!(!is_text_file(Path::new("image.png")), "png");
        assert!(!is_text_file(Path::new("archive.zip")), "zip");
        assert!(!is_text_file(Path::new("binary.bin")), "bin");
    }

    #[test]
    fn is_text_file_case_insensitive() {
        assert!(is_text_file(Path::new("Doc.MD")), "Doc.MD");
        assert!(is_text_file(Path::new("README.TXT")), "README.TXT");
    }

    #[test]
    fn is_text_file_no_extension() {
        assert!(!is_text_file(Path::new("Makefile")), "Makefile");
    }

}
