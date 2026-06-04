// 文件夹监听模块 — 基于 notify crate 监听 inbox 目录，自动导入新文件

use crate::db;
use crate::fs_layout::AppPaths;
use crate::processor;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// 待处理文件 — 等待写入完成后再导入
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

    // 2. 确保 inbox 目录存在
    if let Err(e) = std::fs::create_dir_all(&app_paths.inbox) {
        eprintln!("[watch] 创建 inbox 目录失败: {e}");
        return Err(anyhow::anyhow!("创建 inbox 目录失败: {e}"));
    }

    // 2.5. 初始扫描 — inbox 中已存在且稳定的文件直接处理
    std::thread::sleep(Duration::from_millis(500));

    match std::fs::read_dir(&app_paths.inbox) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if should_skip(&path) || !path.is_file() {
                    continue;
                }
                eprintln!("[watch] 初始扫描: {}", path.display());
                match processor::process_file(&path, app_paths) {
                    Ok(()) => eprintln!("[watch] 导入完成: {}", path.display()),
                    Err(e) => eprintln!("[watch] 导入失败 {}: {:#}", path.display(), e),
                }
            }
        }
        Err(e) => eprintln!("[watch] 初始扫描 inbox 失败: {e}"),
    }

    // 3. 就绪信号 — Node.js 通过 stdout 第一行 JSON 确认启动
    let ready = serde_json::json!({
        "status": "watching",
        "inbox": app_paths.inbox.display().to_string(),
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

    if let Err(e) = watcher.watch(&app_paths.inbox, RecursiveMode::NonRecursive) {
        eprintln!("[watch] 注册监听失败: {e}");
        return Err(anyhow::anyhow!("注册监听失败: {e}"));
    }
    eprintln!("[watch] 开始监听: {}", app_paths.inbox.display());

    // 5. 事件循环 — 等待文件写入完成后再导入
    let mut pending: HashMap<PathBuf, PendingFile> = HashMap::new();
    let mut last_seen: HashMap<PathBuf, Instant> = HashMap::new();
    const STABILITY_MS: Duration = Duration::from_millis(1000);
    const POLL_MS: Duration = Duration::from_millis(500);
    const DEBOUNCE_MS: Duration = Duration::from_millis(800);
    let mut event_count: u64 = 0;

    loop {
        event_count += 1;

        // 定期清理过期记录
        if event_count % 100 == 0 {
            last_seen.retain(|_, t| t.elapsed() < DEBOUNCE_MS);
            pending.retain(|_, p| p.last_seen.elapsed() < STABILITY_MS * 10);
        }

        // 非阻塞接收 notify 事件
        match rx.recv_timeout(POLL_MS) {
            Ok(event) => {
                // 忽略纯目录事件
                if matches!(
                    event.kind,
                    EventKind::Create(notify::event::CreateKind::Folder) | EventKind::Remove(_)
                ) {
                    continue;
                }

                for path in event.paths {
                    if should_skip(&path) {
                        continue;
                    }
                    let now = Instant::now();
                    let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                    pending
                        .entry(path)
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
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // 无新事件，检查 pending 中已稳定的文件
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("[watch] 监听通道关闭，退出");
                break;
            }
        }

        // 处理已稳定的文件（1 秒内无新事件 + 文件大小不变）
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
                continue; // 仍在写入中
            }
            stable.push(path.clone());
        }

        for path in stable {
            pending.remove(&path);

            // debounce 去重
            if let Some(last) = last_seen.get(&path) {
                if now.duration_since(*last) < DEBOUNCE_MS {
                    continue;
                }
            }

            eprintln!("[watch] 检测到稳定文件: {}", path.display());
            last_seen.insert(path.clone(), now);

            match processor::process_file(&path, app_paths) {
                Ok(()) => eprintln!("[watch] 导入完成: {}", path.display()),
                Err(e) => eprintln!("[watch] 导入失败 {}: {:#}", path.display(), e),
            }
        }
    }

    Ok(())
}

/// 跳过临时文件和隐藏文件
fn should_skip(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    // 跳过临时/部分写入文件
    if name.ends_with('~')
        || name.ends_with(".tmp")
        || name.ends_with(".crdownload")
        || name.ends_with(".part")
    {
        return true;
    }

    // 跳过隐藏文件
    if name.starts_with('.') {
        return true;
    }

    // 跳过 MS Office 锁文件
    if name.starts_with("~$") {
        return true;
    }

    false
}
