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

/// 从 CLI args 覆盖 AppPaths 中的路径：
/// --library / --db-path（可选，不传则保持 config 默认值）
/// db_path 额外支持 DATABASE_URL 环境变量作为 fallback
fn merge_cli_paths(args: &[String], app_paths: &AppPaths) -> AppPaths {
    let mut paths = app_paths.clone();

    if let Some(idx) = args.iter().position(|a| a == "--library") {
        if let Some(val) = args.get(idx + 1) {
            paths.library = PathBuf::from(val);
        }
    }
    if let Some(idx) = args.iter().position(|a| a == "--db-path") {
        if let Some(val) = args.get(idx + 1) {
            paths.db_path = PathBuf::from(val);
            return paths;
        }
    }
    // db_path fallback: DATABASE_URL 环境变量
    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        if let Some(p) = db_url.strip_prefix("file:") {
            paths.db_path = PathBuf::from(p);
        }
    }
    paths
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
                let paths = merge_cli_paths(&args, &app_paths);
                match processor::process_file(path, &paths) {
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
                let paths = merge_cli_paths(&args, &app_paths);
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
