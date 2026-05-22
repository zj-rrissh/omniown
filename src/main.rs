mod classifier;
mod db;
mod embedding;
mod fs_layout;
mod processor;
mod storage;
#[cfg(test)]
mod tests;

use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecursiveMode, Result, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use embedding::EmbeddingProvider;
use fs_layout::AppPaths;

const DEBOUNCE_DURATION: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
enum FileTask {
    Upsert(PathBuf),
    Remove(PathBuf),
}

fn is_text_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => processor::ALLOWED_EXTENSIONS.contains(&ext),
        None => false,
    }
}

fn handle_file_remove(path: &Path, app_paths: &AppPaths) {
    let stored_path = path.to_string_lossy().to_string();

    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("⚠️ 打开数据库失败: {}", e);
            return;
        }
    };

    match db::delete_document_by_stored_path(&conn, &stored_path) {
        Ok(true) => println!("🗑️ 已从数据库移除: {}", stored_path),
        Ok(false) => println!("⏭️ 数据库中无此路径记录，跳过: {}", stored_path),
        Err(e) => eprintln!("⚠️ 数据库删除失败 [{}]: {}", stored_path, e),
    }
}

fn print_status(app_paths: &AppPaths) {
    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let total = db::count_documents(&conn).unwrap_or(0);
    let public = db::count_by_folder_type(&conn, "public").unwrap_or(0);
    let private = db::count_by_folder_type(&conn, "private").unwrap_or(0);
    let failed = db::count_by_processing_status(&conn, "failed").unwrap_or(0);
    let pending_embeddings =
        db::count_by_processing_status(&conn, "indexed").unwrap_or(0);

    println!("\nOmniOwn Sentinel started\n");
    println!("Database: {}", app_paths.db_path.display());
    println!("Watching: {}", app_paths.inbox.display());
    println!("Concurrency: 4\n");
    println!("Documents:");
    println!("- total: {}", total);
    println!("- public: {}", public);
    println!("- private: {}", private);
    println!("- failed: {}", failed);
    println!("- pending embeddings: {}\n", pending_embeddings);
}

fn run_search(args: &[String]) {
    let app_paths = AppPaths::new(".");
    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ 无法打开数据库: {}", e);
            return;
        }
    };

    let query = &args[2];
    println!("\nSearch: {}\n", query);

    match db::search_documents(&conn, query, 20) {
        Ok(results) if results.is_empty() => {
            println!("没有找到匹配的文档。\n");
        }
        Ok(results) => {
            for (i, r) in results.iter().enumerate() {
                println!("[{}] {}", i + 1, r.filename);
                println!("Path: {}", r.stored_path);
                println!("Type: {} / {}", r.folder_type, r.category);
                if let Some(snippet) = &r.snippet {
                    println!("Snippet: {}", snippet);
                }
                println!("Rank: {:.2}", r.rank);
                println!();
            }
            println!("共找到 {} 个结果。\n", results.len());
        }
        Err(e) => {
            eprintln!("❌ 搜索失败: {}", e);
        }
    }
}

fn run_embed(args: &[String]) {
    let app_paths = AppPaths::new(".");
    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ 无法打开数据库: {}", e);
            return;
        }
    };

    db::init_embedding_schema(&conn).ok();

    let mut limit = 100usize;
    for i in 0..args.len() {
        if args[i] == "--limit" && i + 1 < args.len() {
            limit = args[i + 1].parse().unwrap_or(100);
        }
    }

    let provider = embedding::MockEmbeddingProvider::new(384);
    println!("🧠 OmniOwn embedding worker\nmodel: {}\nlimit: {}\n", provider.model_name(), limit);

    match embedding::run_embedding_batch(&conn, &provider, limit) {
        Ok(stats) => {
            println!(
                "✅ embedding completed: done={} skipped={} failed={}",
                stats.done, stats.skipped, stats.failed
            );
        }
        Err(e) => eprintln!("❌ embedding 失败: {}", e),
    }
}

fn run_semantic_search(args: &[String]) {
    let app_paths = AppPaths::new(".");
    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ 无法打开数据库: {}", e);
            return;
        }
    };

    let query = &args[2];
    let mut limit = 10usize;
    let mut folder_type: Option<String> = None;

    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--limit" if i + 1 < args.len() => {
                limit = args[i + 1].parse().unwrap_or(10);
                i += 2;
            }
            "--folder" if i + 1 < args.len() => {
                folder_type = Some(args[i + 1].clone());
                i += 2;
            }
            _ => i += 1,
        }
    }

    let provider = embedding::MockEmbeddingProvider::new(384);
    println!("🔎 Semantic search: {}\nmodel: {}\n", query, provider.model_name());

    match embedding::semantic_search(
        &conn,
        &provider,
        query,
        folder_type.as_deref(),
        limit,
    ) {
        Ok(results) if results.is_empty() => {
            println!("没有可搜索的 embedding。请先运行：\ncargo run -- embed\n");
        }
        Ok(results) => {
            for (i, r) in results.iter().enumerate() {
                println!(
                    "{}. score={:.4} [{}/{}] {}",
                    i + 1, r.score, r.folder_type, r.category, r.filename
                );
                println!("   {}\n", r.stored_path);
            }
            println!("共 {} 个结果。\n", results.len());
        }
        Err(e) => eprintln!("❌ 语义搜索失败: {}", e),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 {
        match args[1].as_str() {
            "search" if args.len() >= 3 => {
                run_search(&args);
                return Ok(());
            }
            "embed" => {
                run_embed(&args);
                return Ok(());
            }
            "semantic-search" if args.len() >= 3 => {
                run_semantic_search(&args);
                return Ok(());
            }
            _ => {}
        }
    }

    let app_paths = AppPaths::new(".");

    if let Err(e) = app_paths.init_directories() {
        eprintln!("❌ 目录初始化失败: {}", e);
        return Ok(());
    }
    println!("📁 目录结构初始化完成");

    if let Err(e) = db::init_database(&app_paths.db_path) {
        eprintln!("❌ 数据库初始化失败: {}", e);
        return Ok(());
    }

    print_status(&app_paths);

    println!("👁️ AI 哨兵已启动，正在监控: {}\n", app_paths.inbox.display());

    let last_modify: Arc<Mutex<HashMap<PathBuf, Instant>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let (tx, mut rx) = tokio::sync::mpsc::channel::<FileTask>(1000);

    let app_paths_bg = app_paths.clone();
    tokio::spawn(async move {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(4));

        while let Some(task) = rx.recv().await {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
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
                            Ok(Err(e)) => eprintln!("⚠️ 处理文件失败 [{:?}]: {}", path, e),
                            Err(e) => eprintln!("⚠️ 阻塞任务失败: {}", e),
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
                eprintln!("❌ 监控错误: {:?}", res.err());
                return;
            };

            match event.kind {
                EventKind::Access(_) => {}

                EventKind::Remove(_)
                | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                    for path in event.paths {
                        if is_text_file(&path) {
                            println!("🗑️ 文件已移除: {:?}", path);
                            let _ = tx.blocking_send(FileTask::Remove(path));
                        }
                    }
                }

                EventKind::Create(_)
                | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                    for path in event.paths {
                        if is_text_file(&path) {
                            println!("📄 新文件入队: {:?}", path);
                            let _ = tx.blocking_send(FileTask::Upsert(path));
                        }
                    }
                }

                _ => {
                    let mut map = last_modify.lock().unwrap();
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
                        println!("📝 修改任务入队: {:?}", path);
                        let _ = tx.blocking_send(FileTask::Upsert(path));
                    }
                }
            }
        }
    })?;

    watcher.watch(&app_paths.inbox, RecursiveMode::NonRecursive)?;

    tokio::signal::ctrl_c().await.ok();
    println!("👋 已退出");

    Ok(())
}
