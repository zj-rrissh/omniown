use notify::event::{ModifyKind, RenameMode};
use notify::{EventKind, Event, RecursiveMode, Result, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 防抖窗口：同一文件在此时间内重复触发 Modify 事件将被忽略
const DEBOUNCE_DURATION: Duration = Duration::from_secs(1);

/// 允许处理的文本文件扩展名（小写，无点号）
const ALLOWED_EXTENSIONS: &[&str] = &[
    "txt", "md", "rs", "py", "js", "ts", "json", "yaml", "yml",
    "toml", "cfg", "ini", "conf", "xml", "html", "css", "csv",
    "log", "sh", "bat", "env",
];

/// 判断文件扩展名是否在允许的白名单中
fn is_text_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ALLOWED_EXTENSIONS.contains(&ext),
        None => false,
    }
}

fn main() -> Result<()> {
    // 1. 定义你要监控的"漏斗文件夹"路径
    let watch_path = "./inbox";

    if !Path::new(watch_path).exists() {
        std::fs::create_dir(watch_path).expect("无法创建 inbox 文件夹");
        println!("已自动创建测试文件夹: {}", watch_path);
    }

    println!("👁️ AI 哨兵已启动，正在监控: {}", watch_path);

    // 2. 仅 Modify 事件需要防抖状态
    let last_modify: Arc<Mutex<HashMap<PathBuf, Instant>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // 3. 初始化 Watcher，按事件类型分流
    let mut watcher = notify::recommended_watcher({
        let last_modify = Arc::clone(&last_modify);
        move |res: Result<Event>| {
            match res {
                Ok(event) => match event.kind {
                    // —— Access：直接拦截丢弃 ——
                    EventKind::Access(_) => {}

                    // —— Remove / Move-From：文件被删除或移出监控目录 ——
                    EventKind::Remove(_)
                    | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                        for path in &event.paths {
                            println!("🗑️ 文件已移除: {:?}", path);
                        }
                    }

                    // —— Create / Move-To：新文件进入监控目录 ——
                    EventKind::Create(_)
                    | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                        for path in &event.paths {
                            if !is_text_file(path) {
                                println!("⏭️ 跳过非文本文件: {:?}", path);
                                continue;
                            }
                            println!("📄 新文件: {:?}", path);
                        }
                    }

                    // —— 其他 Modify（内容/元数据修改）：防抖后放行 ——
                    _ => {
                        let mut map = last_modify.lock().unwrap();
                        let now = Instant::now();

                        for path in &event.paths {
                            if !is_text_file(path) {
                                println!("⏭️ 跳过非文本文件: {:?}", path);
                                continue;
                            }

                            if let Some(last) = map.get(path) {
                                if now.duration_since(*last) < DEBOUNCE_DURATION {
                                    continue;
                                }
                            }

                            map.insert(path.clone(), now);
                            println!("📝 文件已修改: {:?}", path);
                        }
                    }
                },
                Err(e) => println!("❌ 监控错误: {:?}", e),
            }
        }
    })?;

    // 4. 绑定路径，非递归监控
    watcher.watch(Path::new(watch_path), RecursiveMode::NonRecursive)?;

    // 5. 阻塞主线程
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
