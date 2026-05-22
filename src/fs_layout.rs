use crate::config::PathsConfig;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,

    pub inbox: PathBuf,

    pub library: PathBuf,
    pub public: PathBuf,
    pub private: PathBuf,

    pub index: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub quarantine: PathBuf,
    pub trash: PathBuf,
    pub config: PathBuf,

    pub db_path: PathBuf,
}

impl AppPaths {
    #[allow(dead_code)]
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();

        let inbox = root.join("inbox");

        let library = root.join("library");
        let public = library.join("public");
        let private = library.join("private");

        let index = root.join("index");
        let cache = root.join("cache");
        let logs = root.join("logs");
        let quarantine = root.join("quarantine");
        let trash = root.join("trash");
        let config = root.join("config");

        let db_path = index.join("omniown.db");

        Self {
            root,
            inbox,
            library,
            public,
            private,
            index,
            cache,
            logs,
            quarantine,
            trash,
            config,
            db_path,
        }
    }

    pub fn init_directories(&self) -> io::Result<()> {
        let dirs = [
            &self.root,
            &self.inbox,
            &self.library,
            &self.public,
            &self.private,
            &self.index,
            &self.cache,
            &self.logs,
            &self.quarantine,
            &self.trash,
            &self.config,
        ];

        for dir in dirs {
            fs::create_dir_all(dir)?;
        }

        Ok(())
    }

    pub fn from_config(cfg: &PathsConfig) -> Self {
        Self {
            root: cfg.root.clone(),
            inbox: cfg.inbox.clone(),
            library: cfg.library.clone(),
            public: cfg.library.join("public"),
            private: cfg.library.join("private"),
            index: cfg.index.clone(),
            cache: cfg.cache.clone(),
            logs: cfg.logs.clone(),
            quarantine: cfg.quarantine.clone(),
            trash: cfg.trash.clone(),
            config: cfg.config_dir.clone(),
            db_path: cfg.database.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn category_dir(&self, folder_type: &str, _category: &str) -> PathBuf {
        match folder_type {
            "private" => self.private.clone(),
            _ => self.public.clone(),
        }
    }
}
