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
    // Intentionally not pub(crate) — used only in #[cfg(test)] modules;
    // production uses AppPaths::from_config().
    #[cfg_attr(not(test), allow(dead_code))]
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

    // Test helper — only used in #[cfg(test)] blocks.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn category_dir(&self, folder_type: &str, _category: &str) -> PathBuf {
        match folder_type {
            "private" => self.private.clone(),
            _ => self.public.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PathsConfig;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_root() -> PathBuf {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "omniown_fs_layout_test_{}_{}",
            std::process::id(),
            counter
        ))
    }

    #[test]
    fn new_creates_correct_path_structure() {
        let root = PathBuf::from("/tmp/test_project");
        let paths = AppPaths::new(&root);

        assert_eq!(paths.root, PathBuf::from("/tmp/test_project"));
        assert_eq!(paths.inbox, root.join("inbox"));
        assert_eq!(paths.library, root.join("library"));
        assert_eq!(paths.public, root.join("library/public"));
        assert_eq!(paths.private, root.join("library/private"));
        assert_eq!(paths.index, root.join("index"));
        assert_eq!(paths.cache, root.join("cache"));
        assert_eq!(paths.logs, root.join("logs"));
        assert_eq!(paths.quarantine, root.join("quarantine"));
        assert_eq!(paths.trash, root.join("trash"));
        assert_eq!(paths.config, root.join("config"));
        assert_eq!(paths.db_path, root.join("index/omniown.db"));
    }

    #[test]
    fn init_directories_creates_all_dirs() {
        let root = temp_root();
        let paths = AppPaths::new(&root);

        assert!(paths.init_directories().is_ok());

        assert!(paths.root.exists());
        assert!(paths.inbox.exists());
        assert!(paths.library.exists());
        assert!(paths.public.exists());
        assert!(paths.private.exists());
        assert!(paths.index.exists());
        assert!(paths.cache.exists());
        assert!(paths.logs.exists());
        assert!(paths.quarantine.exists());
        assert!(paths.trash.exists());
        assert!(paths.config.exists());

        // cleanup
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn init_directories_is_idempotent_when_dirs_exist() {
        let root = temp_root();
        let paths = AppPaths::new(&root);

        // First call
        assert!(paths.init_directories().is_ok());
        // Second call — should not error
        assert!(paths.init_directories().is_ok());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn from_config_maps_all_fields() {
        let cfg = PathsConfig {
            root: PathBuf::from("/custom/root"),
            inbox: PathBuf::from("/custom/inbox"),
            library: PathBuf::from("/custom/lib"),
            index: PathBuf::from("/custom/index"),
            cache: PathBuf::from("/custom/cache"),
            logs: PathBuf::from("/custom/logs"),
            quarantine: PathBuf::from("/custom/quarantine"),
            trash: PathBuf::from("/custom/trash"),
            config_dir: PathBuf::from("/custom/config"),
            database: PathBuf::from("/custom/my.db"),
        };

        let paths = AppPaths::from_config(&cfg);

        assert_eq!(paths.root, PathBuf::from("/custom/root"));
        assert_eq!(paths.inbox, PathBuf::from("/custom/inbox"));
        assert_eq!(paths.library, PathBuf::from("/custom/lib"));
        assert_eq!(paths.public, PathBuf::from("/custom/lib/public"));
        assert_eq!(paths.private, PathBuf::from("/custom/lib/private"));
        assert_eq!(paths.index, PathBuf::from("/custom/index"));
        assert_eq!(paths.cache, PathBuf::from("/custom/cache"));
        assert_eq!(paths.logs, PathBuf::from("/custom/logs"));
        assert_eq!(paths.quarantine, PathBuf::from("/custom/quarantine"));
        assert_eq!(paths.trash, PathBuf::from("/custom/trash"));
        assert_eq!(paths.config, PathBuf::from("/custom/config"));
        assert_eq!(paths.db_path, PathBuf::from("/custom/my.db"));
    }

    #[test]
    fn category_dir_private_returns_private_dir() {
        let root = PathBuf::from("/tmp");
        let paths = AppPaths::new(&root);

        let dir = paths.category_dir("private", "finance");
        assert_eq!(dir, paths.private);
    }

    #[test]
    fn category_dir_public_returns_public_dir() {
        let root = PathBuf::from("/tmp");
        let paths = AppPaths::new(&root);

        let dir = paths.category_dir("public", "notes");
        assert_eq!(dir, paths.public);
    }

    #[test]
    fn category_dir_unknown_folder_type_defaults_to_public() {
        let root = PathBuf::from("/tmp");
        let paths = AppPaths::new(&root);

        let dir = paths.category_dir("unknown", "misc");
        assert_eq!(dir, paths.public);
    }

    #[test]
    fn db_path_is_inside_index_dir() {
        let root = PathBuf::from("/tmp/project");
        let paths = AppPaths::new(&root);

        assert_eq!(paths.db_path.parent(), Some(paths.index.as_path()));
        assert_eq!(
            paths.db_path.file_name().unwrap(),
            std::ffi::OsStr::new("omniown.db")
        );
    }
}
