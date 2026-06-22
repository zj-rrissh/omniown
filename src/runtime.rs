use crate::config::AppConfig;
use crate::extractor::{self, ExtractedText};
use crate::fs_layout::AppPaths;
use crate::processor::{self, IndexResult};
use crate::{mcp, watch};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct OmniownKernel {
    pub config: AppConfig,
    pub paths: AppPaths,
}

impl OmniownKernel {
    pub fn load() -> Self {
        let initial_root = std::env::var("OMNIOWN_ROOT").unwrap_or_else(|_| ".".to_string());
        Self::load_from_root(initial_root)
    }

    pub fn load_from_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        let config_dir = if root.join("omniown.toml").exists() {
            root.to_path_buf()
        } else {
            root.join("config")
        };
        let config = AppConfig::load(&config_dir);
        Self::from_config(config)
    }

    pub fn from_config(config: AppConfig) -> Self {
        let paths = AppPaths::from_config(&config.paths);
        Self { config, paths }
    }

    pub fn with_paths(config: AppConfig, paths: AppPaths) -> Self {
        Self { config, paths }
    }

    pub fn process_file(&self, path: &Path) -> anyhow::Result<()> {
        processor::process_file(path, &self.paths)
    }

    pub fn extract_text(&self, path: &Path) -> anyhow::Result<ExtractedText> {
        extractor::extract_text(path)
    }

    pub fn index_file_in_place(&self, path: &Path) -> anyhow::Result<IndexResult> {
        processor::index_file_in_place(path, &self.paths)
    }

    pub fn run_watch(&self) -> anyhow::Result<()> {
        watch::run_watch(&self.paths, &self.paths.db_path)
    }

    pub fn run_mcp(&self) -> anyhow::Result<()> {
        mcp::run_mcp(&self.config, &self.paths)
    }
}

/// 从 CLI args 覆盖 AppPaths 中的路径：
/// --library / --db-path（可选，不传则保持 config 默认值）
/// db_path 额外支持 DATABASE_URL 环境变量作为 fallback
pub fn merge_cli_paths(args: &[String], app_paths: &AppPaths) -> AppPaths {
    let mut paths = app_paths.clone();

    if let Some(val) = args
        .iter()
        .position(|a| a == "--library")
        .and_then(|idx| args.get(idx + 1))
    {
        paths.library = PathBuf::from(val);
    }
    if let Some(val) = args
        .iter()
        .position(|a| a == "--db-path")
        .and_then(|idx| args.get(idx + 1))
    {
        paths.db_path = PathBuf::from(val);
        return paths;
    }
    // db_path fallback: DATABASE_URL 环境变量
    if let Some(p) = std::env::var("DATABASE_URL")
        .ok()
        .and_then(|url| url.strip_prefix("file:").map(String::from))
    {
        paths.db_path = PathBuf::from(p);
    }
    paths
}
