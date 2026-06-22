use omniown_core::config;
use omniown_core::runtime::{OmniownKernel, merge_cli_paths};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2 && args[1] == "config-example" {
        config::print_example_config();
        return;
    }

    let kernel = OmniownKernel::load();

    if args.len() >= 2 {
        match args[1].as_str() {
            "process" if args.len() >= 3 => {
                let path = Path::new(&args[2]);
                let paths = merge_cli_paths(&args, &kernel.paths);
                let kernel = OmniownKernel::with_paths(kernel.config.clone(), paths);
                match kernel.process_file(path) {
                    Ok(()) => {}
                    Err(e) => eprintln!("处理失败: {e}"),
                }
                return;
            }
            "extract" if args.len() >= 3 => {
                let path = Path::new(&args[2]);
                match kernel.extract_text(path) {
                    Ok(extracted) => println!("{}", extracted.text),
                    Err(e) => eprintln!("提取失败: {e}"),
                }
                return;
            }
            "mcp" => {
                if let Err(e) = kernel.run_mcp() {
                    eprintln!("\u{274c} MCP server error: {e:#}");
                }
                return;
            }
            "watch" => {
                let paths = merge_cli_paths(&args, &kernel.paths);
                let kernel = OmniownKernel::with_paths(kernel.config.clone(), paths);
                if let Err(e) = kernel.run_watch() {
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
    use omniown_core::processor;
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
