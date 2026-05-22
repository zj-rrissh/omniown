use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// OmniOwn 的目录结构管理
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,

    pub inbox: PathBuf,

    pub library: PathBuf,
    pub public: PathBuf,
    pub public_notes: PathBuf,
    pub public_docs: PathBuf,
    pub public_code: PathBuf,
    pub public_misc: PathBuf,

    pub private: PathBuf,
    pub private_journal: PathBuf,
    pub private_finance: PathBuf,
    pub private_identity: PathBuf,
    pub private_misc: PathBuf,

    pub index: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub quarantine: PathBuf,
    pub trash: PathBuf,
    pub config: PathBuf,

    pub db_path: PathBuf,
}

impl AppPaths {
    /// 根据项目根目录生成所有路径
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();

        let inbox = root.join("inbox");

        let library = root.join("library");

        let public = library.join("public");
        let public_notes = public.join("notes");
        let public_docs = public.join("docs");
        let public_code = public.join("code");
        let public_misc = public.join("misc");

        let private = library.join("private");
        let private_journal = private.join("journal");
        let private_finance = private.join("finance");
        let private_identity = private.join("identity");
        let private_misc = private.join("misc");

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
            public_notes,
            public_docs,
            public_code,
            public_misc,

            private,
            private_journal,
            private_finance,
            private_identity,
            private_misc,

            index,
            cache,
            logs,
            quarantine,
            trash,
            config,

            db_path,
        }
    }

    /// 创建所有必要目录
    pub fn init_directories(&self) -> io::Result<()> {
        let dirs = [
            &self.root,

            &self.inbox,

            &self.library,
            &self.public,
            &self.public_notes,
            &self.public_docs,
            &self.public_code,
            &self.public_misc,

            &self.private,
            &self.private_journal,
            &self.private_finance,
            &self.private_identity,
            &self.private_misc,

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

    /// 根据 folder_type + category 返回目标目录
    ///
    /// 例：
    /// public + notes   => library/public/notes
    /// private + finance => library/private/finance
    pub fn category_dir(&self, folder_type: &str, category: &str) -> PathBuf {
        match (folder_type, category) {
            ("public", "notes") => self.public_notes.clone(),
            ("public", "docs") => self.public_docs.clone(),
            ("public", "code") => self.public_code.clone(),
            ("public", "misc") => self.public_misc.clone(),

            ("private", "journal") => self.private_journal.clone(),
            ("private", "finance") => self.private_finance.clone(),
            ("private", "identity") => self.private_identity.clone(),
            ("private", "misc") => self.private_misc.clone(),

            ("public", _) => self.public_misc.clone(),
            ("private", _) => self.private_misc.clone(),

            _ => self.public_misc.clone(),
        }
    }
}