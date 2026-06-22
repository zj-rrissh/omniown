// 文件夹监听模块 — 基于 notify crate 监听 library 目录，增删自动同步数据库

use crate::db;
use crate::fs_layout::AppPaths;
use crate::processor;
use notify::event::{CreateKind, EventKind, RemoveKind};
use notify::{Event, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// 待处理文件 — 等待写入完成后再索引
struct PendingFile {
    last_seen: Instant,
    last_size: u64,
}

#[derive(Clone, PartialEq, Eq)]
struct FileFingerprint {
    len: u64,
    modified: Option<SystemTime>,
}

struct ProcessedFile {
    seen_at: Instant,
    fingerprint: FileFingerprint,
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
    let mut watcher =
        notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
            Ok(event) => {
                let _ = tx.send(event);
            }
            Err(e) => eprintln!("[watch] 监听错误: {e}"),
        })?;

    if let Err(e) = watcher.watch(&app_paths.library, RecursiveMode::Recursive) {
        eprintln!("[watch] 注册监听失败: {e}");
        return Err(anyhow::anyhow!("注册监听失败: {e}"));
    }
    eprintln!("[watch] 开始监听: {}", app_paths.library.display());

    // 5. 事件循环
    let mut pending: HashMap<PathBuf, PendingFile> = HashMap::new();
    let mut last_seen: HashMap<PathBuf, Instant> = HashMap::new();
    let mut processed: HashMap<PathBuf, ProcessedFile> = HashMap::new();
    const STABILITY_MS: Duration = Duration::from_millis(1000);
    const POLL_MS: Duration = Duration::from_millis(500);
    const DEBOUNCE_MS: Duration = Duration::from_millis(800);
    const PROCESSED_TTL: Duration = Duration::from_secs(30);
    let mut event_count: u64 = 0;

    loop {
        event_count += 1;

        if event_count.is_multiple_of(100) {
            last_seen.retain(|_, t| t.elapsed() < DEBOUNCE_MS);
            pending.retain(|_, p| p.last_seen.elapsed() < STABILITY_MS * 10);
            processed.retain(|_, p| p.seen_at.elapsed() < PROCESSED_TTL);
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
                    if should_skip(path) {
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

            if last_seen
                .get(&path)
                .is_some_and(|last| now.duration_since(*last) < DEBOUNCE_MS)
            {
                continue;
            }

            if was_recently_processed(&processed, &path, PROCESSED_TTL) {
                continue;
            }

            last_seen.insert(path.clone(), now);

            match processor::index_file_in_place(&path, app_paths) {
                Ok(result) => {
                    remember_processed(&mut processed, &path);

                    // 文件被移动到新路径 → 将新路径也加入 last_seen，防止
                    // 移动产生的事件导致同一文件被重复处理
                    if let Some(ref moved_to) = result.moved_to {
                        last_seen.insert(moved_to.clone(), now);
                        remember_processed(&mut processed, moved_to);
                        eprintln!("[watch] 索引完成（已归类）: {}", moved_to.display());
                    } else if result.changed {
                        eprintln!("[watch] 索引完成: {}", path.display());
                    }
                }
                Err(e) => eprintln!("[watch] 索引失败 {}: {:#}", path.display(), e),
            }
        }
    }

    Ok(())
}

/// 两阶段扫描 library 目录：先收集所有文件路径，再逐个处理。
/// 避免边扫描边处理时，文件移动导致递归扫描重复处理同一文件。
fn scan_library(app_paths: &AppPaths, dir: &Path) {
    let mut files: Vec<PathBuf> = Vec::new();
    collect_files(dir, &mut files);

    for path in &files {
        // 文件可能在之前的处理中被移动/删除
        if !path.exists() {
            continue;
        }
        match processor::index_file_in_place(path, app_paths) {
            Ok(result) => {
                if let Some(ref moved_to) = result.moved_to {
                    eprintln!("[watch] 索引完成（已归类）: {}", moved_to.display());
                } else if result.changed {
                    eprintln!("[watch] 索引完成: {}", path.display());
                }
            }
            Err(e) => eprintln!("[watch] 索引失败 {}: {:#}", path.display(), e),
        }
    }
}

/// 递归收集目录下所有文件路径（不处理，仅收集）
fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
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
            collect_files(&path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

/// 文件被删除 → 从数据库移除记录
fn handle_remove(path: &Path, app_paths: &AppPaths) {
    // 使用与 index_file_in_place 相同的路径计算逻辑，确保删除能匹配到写入时的记录
    let stored_path = processor::stored_path_for_db(path, app_paths);
    let stored_path_str = stored_path.to_string_lossy().to_string();

    let conn = match rusqlite::Connection::open(&app_paths.db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[watch] 删除记录时打开数据库失败: {e}");
            return;
        }
    };

    match db::delete_document_by_stored_path(&conn, &stored_path_str) {
        Ok(true) => eprintln!("[watch] 已删除记录: {}", stored_path_str),
        Ok(false) => eprintln!("[watch] 未找到对应记录（可能已被清理）: {}", stored_path_str),
        Err(e) => eprintln!("[watch] 删除记录失败 {}: {}", stored_path_str, e),
    }
}

fn file_fingerprint(path: &Path) -> Option<FileFingerprint> {
    let metadata = path.metadata().ok()?;
    if !metadata.is_file() {
        return None;
    }
    Some(FileFingerprint {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

fn processed_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn was_recently_processed(
    processed: &HashMap<PathBuf, ProcessedFile>,
    path: &Path,
    ttl: Duration,
) -> bool {
    let Some(fingerprint) = file_fingerprint(path) else {
        return false;
    };

    processed
        .get(&processed_key(path))
        .is_some_and(|entry| entry.seen_at.elapsed() < ttl && entry.fingerprint == fingerprint)
}

fn remember_processed(processed: &mut HashMap<PathBuf, ProcessedFile>, path: &Path) {
    let Some(fingerprint) = file_fingerprint(path) else {
        return;
    };

    processed.insert(
        processed_key(path),
        ProcessedFile {
            seen_at: Instant::now(),
            fingerprint,
        },
    );
}

fn should_skip(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

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
