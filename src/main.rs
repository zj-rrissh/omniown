mod db;

use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecursiveMode, Result, Watcher};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DEBOUNCE_DURATION: Duration = Duration::from_secs(1);

const ALLOWED_EXTENSIONS: &[&str] = &["txt", "md"];

#[derive(Debug, Clone)]
enum FileTask {
    Upsert(PathBuf),
    Remove(PathBuf),
}

fn is_text_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ALLOWED_EXTENSIONS.contains(&ext),
        None => false,
    }
}

fn handle_file_upsert(path: &Path) {
    let filename = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => {
            eprintln!("⚠️ 无法解析文件名: {:?}", path);
            return;
        }
    };

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("⚠️ 读取文件失败 [{}]: {}", filename, e);
            return;
        }
    };

    let conn = match Connection::open("omniown.db") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("⚠️ 打开数据库失败: {}", e);
            return;
        }
    };

    if let Err(e) = db::upsert_document(&conn, filename, &content) {
        eprintln!("⚠️ 写入数据库失败 [{}]: {}", filename, e);
    }
}

fn handle_file_remove(path: &Path) {
    let filename = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return,
    };

    let conn = match Connection::open("omniown.db") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("⚠️ 打开数据库失败: {}", e);
            return;
        }
    };

    match db::delete_document(&conn, filename) {
        Ok(true) => println!("🗑️ 已从数据库移除: {}", filename),
        Ok(false) => println!("⏭️ 数据库中无此文件记录，跳过: {}", filename),
        Err(e) => eprintln!("⚠️ 数据库删除失败 [{}]: {}", filename, e),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = db::init_database() {
        eprintln!("❌ 数据库初始化失败: {}", e);
        return Ok(());
    }

    let watch_path = "./inbox";

    if !Path::new(watch_path).exists() {
        std::fs::create_dir(watch_path).expect("无法创建 inbox 文件夹");
        println!("已自动创建测试文件夹: {}", watch_path);
    }

    println!("👁️ AI 哨兵已启动，正在监控: {}\n", watch_path);

    let last_modify: Arc<Mutex<HashMap<PathBuf, Instant>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let (tx, mut rx) = tokio::sync::mpsc::channel::<FileTask>(1000);

    // 后台消费者：最多同时处理 8 个文件
    tokio::spawn(async move {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(8));

        while let Some(task) = rx.recv().await {
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            tokio::spawn(async move {
                let _permit = permit;

                match task {
                    FileTask::Upsert(path) => {
                        let _ = tokio::task::spawn_blocking(move || {
                            handle_file_upsert(&path);
                        })
                        .await;
                    }
                    FileTask::Remove(path) => {
                        let _ = tokio::task::spawn_blocking(move || {
                            handle_file_remove(&path);
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

                        if let Some(last) = map.get(&path) {
                            if now.duration_since(*last) < DEBOUNCE_DURATION {
                                continue;
                            }
                        }

                        map.insert(path.clone(), now);
                        println!("📝 修改任务入队: {:?}", path);
                        let _ = tx.blocking_send(FileTask::Upsert(path));
                    }
                }
            }
        }
    })?;

    watcher.watch(Path::new(watch_path), RecursiveMode::NonRecursive)?;

    tokio::signal::ctrl_c().await.ok();
    println!("👋 已退出");

    Ok(())
}
