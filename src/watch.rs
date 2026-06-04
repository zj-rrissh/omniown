// 文件夹监听模块 — 基于 notify crate 监听 library 目录，增删自动同步数据库

use crate::db;
use crate::fs_layout::AppPaths;
use crate::processor;
use notify::event::{CreateKind, EventKind, RemoveKind};
use notify::{Event, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// 待处理文件 — 等待写入完成后再索引
struct PendingFile {
    last_seen: Instant,
    last_size: u64,
}

/// 启动文件夹监听。阻塞运行，直到 channel 断开或出错。
pub fn run_watch(app_paths: &AppPaths, db_path: &Path) -> anyhow::Result<()> {
    // 1. 确保数据库表存在（IF NOT EXISTS，幂等安全）
    if let Err(e) = db::init_database(db_path) {
        eprintln!("[watch] 数据库初始化失败: {e}");
        return Err(anyhow::anyhow!("数据库初始化失败: {e}"));
    }

    // 2. 确保 library 目录存在
    if let Err(e) = std::fs::create_dir_all(&app_paths.library) {
        eprintln!("[watch] 创建 library 目录失败: {e}");
        return Err(anyhow::anyhow!("创建 library 目录失败: {e}"));
    }
    if let Err(e) = std::fs::create_dir_all(app_paths.library.join("public")) {
        eprintln!("[watch] 创建 library/public 失败: {e}");
    }
    if let Err(e) = std::fs::create_dir_all(app_paths.library.join("private")) {
        eprintln!("[watch] 创建 library/private 失败: {e}");
    }

    // 2.5. 初始扫描 — library 中已有文件（递归），跳过临时文件和 public/private 目录自身
    std::thread::sleep(Duration::from_millis(500));
    scan_library(app_paths, &app_paths.library);

    // 3. 就绪信号 — Node.js 通过 stdout 第一行 JSON 确认启动
    let ready = serde_json::json!({
        "status": "watching",
        "library": app_paths.library.display().to_string(),
        "db_path": db_path.display().to_string()
    });
    println!("{}", ready);

    // 4. 设置文件系统监听
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        match res {
            Ok(event) => {
                let _ = tx.send(event);
            }
            Err(e) => eprintln!("[watch] 监听错误: {e}"),
        }
    })?;

    if let Err(e) = watcher.watch(&app_paths.library, RecursiveMode::Recursive) {
        eprintln!("[watch] 注册监听失败: {e}");
        return Err(anyhow::anyhow!("注册监听失败: {e}"));
    }
    eprintln!("[watch] 开始监听: {}", app_paths.library.display());

    // 5. 事件循环
    let mut pending: HashMap<PathBuf, PendingFile> = HashMap::new();
    let mut last_seen: HashMap<PathBuf, Instant> = HashMap::new();
    const STABILITY_MS: Duration = Duration::from_millis(1000);
    const POLL_MS: Duration = Duration::from_millis(500);
    const DEBOUNCE_MS: Duration = Duration::from_millis(800);
    let mut event_count: u64 = 0;

    loop {
        event_count += 1;

        if event_count % 100 == 0 {
            last_seen.retain(|_, t| t.elapsed() < DEBOUNCE_MS);
            pending.retain(|_, p| p.last_seen.elapsed() < STABILITY_MS * 10);
        }

        match rx.recv_timeout(POLL_MS) {
            Ok(event) => {
                match event.kind {
                    // 文件被删除 → 同步删除 DB 记录，同时清理 pending 防止后续误索引
                    EventKind::Remove(RemoveKind::File) | EventKind::Remove(RemoveKind::Any) => {
                        for path in &event.paths {
                            if should_skip(path) {
                                continue;
                            }
                            pending.remove(path);
                            last_seen.remove(path);
                            handle_remove(path, app_paths);
                        }
                        continue;
                    }
                    // 目录创建/删除 → 忽略
                    EventKind::Create(CreateKind::Folder) | EventKind::Remove(_) => {
                        continue;
                    }
                    _ => {}
                }

                for path in &event.paths {
                    if should_skip(&path) {
                        continue;
                    }
                    let now = Instant::now();
                    let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                    pending
                        .entry(path.to_path_buf())
                        .and_modify(|p| {
                            p.last_seen = now;
                            p.last_size = size;
                        })
                        .or_insert(PendingFile {
                            last_seen: now,
                            last_size: size,
                        });
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("[watch] 监听通道关闭，退出");
                break;
            }
        }

        // 处理已稳定的文件
        let now = Instant::now();
        let mut stable: Vec<PathBuf> = Vec::new();

        for (path, info) in &pending {
            if now.duration_since(info.last_seen) < STABILITY_MS {
                continue;
            }
            if !path.is_file() {
                stable.push(path.clone());
                continue;
            }
            let current_size = path.metadata().map(|m| m.len()).unwrap_or(0);
            if current_size != info.last_size {
                continue;
            }
            stable.push(path.clone());
        }

        for path in stable {
            pending.remove(&path);

            // 文件在 pending 期间被删除 → 同步清理 DB
            if !path.exists() {
                handle_remove(&path, app_paths);
                continue;
            }

            if let Some(last) = last_seen.get(&path) {
                if now.duration_since(*last) < DEBOUNCE_MS {
                    continue;
                }
            }

            eprintln!("[watch] 检测到稳定文件: {}", path.display());
            last_seen.insert(path.clone(), now);

            match processor::index_file_in_place(&path, app_paths) {
                Ok(()) => eprintln!("[watch] 索引完成: {}", path.display()),
                Err(e) => eprintln!("[watch] 索引失败 {}: {:#}", path.display(), e),
            }
        }
    }

    Ok(())
}

/// 递归扫描 library 目录，索引已有文件
fn scan_library(app_paths: &AppPaths, dir: &Path) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if should_skip(&path) {
            continue;
        }
        if path.is_dir() {
            // 跳过 public/private 自身的递归扫描起点（但不跳过其内容）
            scan_library(app_paths, &path);
        } else if path.is_file() {
            eprintln!("[watch] 初始扫描: {}", path.display());
            match processor::index_file_in_place(&path, app_paths) {
                Ok(()) => eprintln!("[watch] 索引完成: {}", path.display()),
                Err(e) => eprintln!("[watch] 索引失败 {}: {:#}", path.display(), e),
            }
        }
    }
}

/// 文件被删除 → 从数据库移除记录
fn handle_remove(path: &Path, app_paths: &AppPaths) {
    // 使用绝对路径的 root，避免相对路径 strip_prefix 失败
    let abs_root = app_paths.root.canonicalize().unwrap_or_else(|_| app_paths.root.clone());
    let stored_path = path
        .strip_prefix(&abs_root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string());

    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[watch] 删除记录时打开数据库失败: {e}");
            return;
        }
    };

    match db::delete_document_by_stored_path(&conn, &stored_path) {
        Ok(true) => eprintln!("[watch] 已删除记录: {}", stored_path),
        Ok(false) => {} // 记录不存在，正常
        Err(e) => eprintln!("[watch] 删除记录失败 {}: {}", stored_path, e),
    }
}

fn should_skip(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if name.ends_with('~')
        || name.ends_with(".tmp")
        || name.ends_with(".crdownload")
        || name.ends_with(".part")
    {
        return true;
    }
    if name.starts_with('.') {
        return true;
    }
    if name.starts_with("~$") {
        return true;
    }

    false
}
