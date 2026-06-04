mod config;
mod db;
mod extractor;
mod fs_layout;
mod mcp;
mod processor;
mod watch;

use config::AppConfig;
use fs_layout::AppPaths;
use std::path::{Path, PathBuf};

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

/// 解析 watch 子命令的数据库路径：
/// 1. --db-path CLI 参数
/// 2. DATABASE_URL 环境变量（解析 file: 前缀）
/// 3. 默认路径（来自 omniown.toml）
fn resolve_watch_db_path(args: &[String], app_paths: &AppPaths) -> PathBuf {
    if let Some(idx) = args.iter().position(|a| a == "--db-path") {
        if let Some(val) = args.get(idx + 1) {
            return PathBuf::from(val);
        }
    }
    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        if let Some(path) = db_url.strip_prefix("file:") {
            return PathBuf::from(path);
        }
    }
    app_paths.db_path.clone()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2 && args[1] == "config-example" {
        config::print_example_config();
        return;
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
                return;
            }
            "extract" if args.len() >= 3 => {
                let path = Path::new(&args[2]);
                match extractor::extract_text(path) {
                    Ok(extracted) => println!("{}", extracted.text),
                    Err(e) => eprintln!("提取失败: {e}"),
                }
                return;
            }
            "mcp" => {
                if let Err(e) = mcp::run_mcp(&config, &app_paths) {
                    eprintln!("\u{274c} MCP server error: {e:#}");
                }
                return;
            }
            "watch" => {
                let db_path = resolve_watch_db_path(&args, &app_paths);
                let mut paths = app_paths.clone();
                paths.db_path = db_path;
                if let Err(e) = watch::run_watch(&paths, &paths.db_path.clone()) {
                    eprintln!("watch 失败: {e:#}");
                }
                return;
            }
            _ => {
                eprintln!("未知命令: {}", args[1]);
                eprintln!("用法: omniown <command> [args]");
                eprintln!("命令: process, extract, watch, mcp, config-example");
                return;
            }
        }
    }

    eprintln!("用法: omniown <command> [args]");
    eprintln!("命令: process, extract, watch, mcp, config-example");
}

#[cfg(test)]
mod main_tests {
    use super::processor;
    use std::path::Path;

    fn is_text_file(path: &Path) -> bool {
        processor::is_supported_file(path)
    }

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
