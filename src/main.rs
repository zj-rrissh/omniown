use notify::{Watcher, RecursiveMode, Result};
use std::path::Path;
use std::time::Duration;

fn main() -> Result<()> {
    // 1. 定义你要监控的“漏斗文件夹”路径
    // 建议在当前项目下新建一个 inbox 文件夹用于测试
    let watch_path = "./inbox"; 
    
    // 确保测试文件夹存在
    if !Path::new(watch_path).exists() {
        std::fs::create_dir(watch_path).expect("无法创建 inbox 文件夹");
        println!("已自动创建测试文件夹: {}", watch_path);
    }

    println!("👁️ AI 哨兵已启动，正在监控: {}", watch_path);

    // 2. 初始化 Watcher 并配置事件回调
    let mut watcher = notify::recommended_watcher(|res| {
        match res {
            Ok(event) => {
                // 在这里可以细化对特定事件（如新建、修改）的捕获
                println!("📦 检测到文件变动: {:?}", event);
            },
            Err(e) => println!("❌ 监控错误: {:?}", e),
        }
    })?;

    // 3. 绑定路径，设置为非递归监控（MVP 阶段保持简单，只监控单层目录）
    watcher.watch(Path::new(watch_path), RecursiveMode::NonRecursive)?;

    // 4. 阻塞主线程，保持程序持续运行
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}