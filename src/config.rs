use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ---- PathsConfig ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    #[serde(default = "default_root")]
    pub root: PathBuf,
    #[serde(default = "default_library")]
    pub library: PathBuf,
    #[serde(default = "default_index")]
    pub index: PathBuf,
    #[serde(default = "default_cache")]
    pub cache: PathBuf,
    #[serde(default = "default_logs")]
    pub logs: PathBuf,
    #[serde(default = "default_quarantine")]
    pub quarantine: PathBuf,
    #[serde(default = "default_trash")]
    pub trash: PathBuf,
    #[serde(default = "default_config_dir")]
    pub config_dir: PathBuf,
    #[serde(default = "default_database")]
    pub database: PathBuf,
}

fn default_root() -> PathBuf {
    PathBuf::from(".")
}
fn default_library() -> PathBuf {
    PathBuf::from("library")
}
fn default_index() -> PathBuf {
    PathBuf::from("index")
}
fn default_cache() -> PathBuf {
    PathBuf::from("cache")
}
fn default_logs() -> PathBuf {
    PathBuf::from("logs")
}
fn default_quarantine() -> PathBuf {
    PathBuf::from("quarantine")
}
fn default_trash() -> PathBuf {
    PathBuf::from("trash")
}
fn default_config_dir() -> PathBuf {
    PathBuf::from("config")
}
fn default_database() -> PathBuf {
    PathBuf::from("index/omniown.db")
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            root: default_root(),
            library: default_library(),
            index: default_index(),
            cache: default_cache(),
            logs: default_logs(),
            quarantine: default_quarantine(),
            trash: default_trash(),
            config_dir: default_config_dir(),
            database: default_database(),
        }
    }
}

impl PathsConfig {
    pub fn resolve(mut self) -> Self {
        if self.root.as_os_str().is_empty() {
            self.root = default_root();
        }
        if self.library.as_os_str().is_empty() {
            self.library = default_library();
        }
        if self.index.as_os_str().is_empty() {
            self.index = default_index();
        }
        if self.cache.as_os_str().is_empty() {
            self.cache = default_cache();
        }
        if self.logs.as_os_str().is_empty() {
            self.logs = default_logs();
        }
        if self.quarantine.as_os_str().is_empty() {
            self.quarantine = default_quarantine();
        }
        if self.trash.as_os_str().is_empty() {
            self.trash = default_trash();
        }
        if self.config_dir.as_os_str().is_empty() {
            self.config_dir = default_config_dir();
        }
        if self.database.as_os_str().is_empty() {
            self.database = default_database();
        }

        if let Ok(env_root) = std::env::var("OMNIOWN_ROOT") {
            self.root = PathBuf::from(env_root);
        }
        let root = self.root.clone();
        self.library = resolve_against(&root, &self.library);
        self.index = resolve_against(&root, &self.index);
        self.cache = resolve_against(&root, &self.cache);
        self.logs = resolve_against(&root, &self.logs);
        self.quarantine = resolve_against(&root, &self.quarantine);
        self.trash = resolve_against(&root, &self.trash);
        self.config_dir = resolve_against(&root, &self.config_dir);
        self.database = resolve_against(&root, &self.database);
        self
    }
}

fn resolve_against(root: &Path, p: &Path) -> PathBuf {
    if p.is_relative() {
        root.join(p)
    } else {
        p.to_path_buf()
    }
}

// ---- SearchConfig ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default = "default_search_limit")]
    pub default_limit: usize,
    #[serde(default = "default_fts_enabled")]
    pub fts_enabled: bool,
}

fn default_search_limit() -> usize {
    20
}
fn default_fts_enabled() -> bool {
    true
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_limit: default_search_limit(),
            fts_enabled: default_fts_enabled(),
        }
    }
}

// ---- AiConfig ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// API base URL. Defaults to OpenAI: https://api.openai.com/v1
    #[serde(default = "default_ai_base_url")]
    pub base_url: String,
    /// Model name, e.g. "gpt-4o-mini" or "qwen2.5:7b" for Ollama
    #[serde(default = "default_ai_model")]
    pub model: String,
    /// API key. Required for OpenAI; leave empty for local providers (Ollama).
    #[serde(default)]
    pub api_key: String,
}

fn default_ai_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_ai_model() -> String {
    "gpt-4o-mini".to_string()
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            base_url: default_ai_base_url(),
            model: default_ai_model(),
            api_key: String::new(),
        }
    }
}

// ---- AppConfig ----

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub ai: AiConfig,
}

impl AppConfig {
    pub fn load(config_dir: &Path) -> Self {
        let mut config = Self::default();

        let config_path = config_dir.join("omniown.toml");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            match toml::from_str::<AppConfig>(&content) {
                Ok(file_config) => config = file_config,
                Err(e) => {
                    eprintln!(
                        "\u{26a0}\u{fe0f}  配置文件解析失败 {}: {e}, 使用默认配置",
                        config_path.display()
                    );
                }
            }
        }

        config.paths = config.paths.resolve();
        config.apply_env_overrides();
        config
    }

    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("OMNIOWN_DB_PATH") {
            self.paths.database = PathBuf::from(val);
        }
    }
}

pub fn print_example_config() {
    let config = AppConfig::default();
    match toml::to_string_pretty(&config) {
        Ok(toml_str) => println!("{toml_str}"),
        Err(e) => eprintln!("配置序列化失败: {e}"),
    }
}

// ---- tests ----

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::{Mutex, MutexGuard};

    const ENV_KEYS: &[&str] = &["OMNIOWN_ROOT", "OMNIOWN_DB_PATH"];

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        _lock: MutexGuard<'static, ()>,
        saved: Vec<(&'static str, Option<String>)>,
    }

    impl EnvGuard {
        fn new() -> Self {
            let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let saved = ENV_KEYS
                .iter()
                .map(|&key| (key, std::env::var(key).ok()))
                .collect();

            for key in ENV_KEYS {
                unsafe { std::env::remove_var(key) };
            }

            Self { _lock: lock, saved }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.saved {
                match value {
                    Some(value) => unsafe { std::env::set_var(key, value) },
                    None => unsafe { std::env::remove_var(key) },
                }
            }
        }
    }

    #[test]
    fn config_default_values() {
        let config = AppConfig::default();
        assert_eq!(config.paths.root, PathBuf::from("."));
        assert_eq!(config.search.default_limit, 20);
        assert!(config.search.fts_enabled);
    }

    #[test]
    fn config_load_from_file() {
        let _env = EnvGuard::new();
        let id = std::process::id();
        let dir = std::env::temp_dir().join(format!("omniown_cfg_{id}"));
        fs::create_dir_all(&dir).unwrap();
        let toml_content = r#"
[paths]
root = "/tmp/test_root"

[search]
default_limit = 50
fts_enabled = false
"#;
        let config_path = dir.join("omniown.toml");
        let mut f = fs::File::create(&config_path).unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        let config = AppConfig::load(&dir);
        assert_eq!(config.paths.root, PathBuf::from("/tmp/test_root"));
        assert_eq!(config.search.default_limit, 50);
        assert!(!config.search.fts_enabled);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn config_load_file_not_found_uses_defaults() {
        let _env = EnvGuard::new();
        let id = std::process::id();
        let dir = std::env::temp_dir().join(format!("omniown_cfg_missing_{id}"));
        fs::create_dir_all(&dir).unwrap();
        let config = AppConfig::load(&dir);
        assert_eq!(config.paths.root, PathBuf::from("."));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn config_env_overrides_root() {
        let _env = EnvGuard::new();
        let id = std::process::id();
        let dir = std::env::temp_dir().join(format!("omniown_env_root_{id}"));
        fs::create_dir_all(&dir).unwrap();
        let toml_content = r#"[paths]
root = "relative_root"
"#;
        let config_path = dir.join("omniown.toml");
        let mut f = fs::File::create(&config_path).unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        unsafe { std::env::set_var("OMNIOWN_ROOT", "/env/override/root") };
        let config = AppConfig::load(&dir);
        assert_eq!(config.paths.root, PathBuf::from("/env/override/root"));
        unsafe { std::env::remove_var("OMNIOWN_ROOT") };

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn config_env_overrides_database() {
        let _env = EnvGuard::new();
        let mut config = AppConfig::default();
        unsafe { std::env::set_var("OMNIOWN_DB_PATH", "/custom/db.sqlite") };
        config.apply_env_overrides();
        assert_eq!(config.paths.database, PathBuf::from("/custom/db.sqlite"));
        unsafe { std::env::remove_var("OMNIOWN_DB_PATH") };
    }

    #[test]
    fn config_paths_resolve_relative() {
        let _env = EnvGuard::new();
        let paths = PathsConfig::default().resolve();
        assert_eq!(paths.library, PathBuf::from("./library"));
        assert_eq!(paths.database, PathBuf::from("./index/omniown.db"));
    }

    #[test]
    fn config_paths_resolve_absolute_root_in_file() {
        let _env = EnvGuard::new();
        let mut paths = PathsConfig::default();
        paths.root = PathBuf::from("/data");
        let resolved = paths.resolve();
        assert_eq!(resolved.root, PathBuf::from("/data"));
        assert_eq!(resolved.library, PathBuf::from("/data/library"));
        assert_eq!(resolved.database, PathBuf::from("/data/index/omniown.db"));
    }

    #[test]
    fn config_example_roundtrip() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.paths.root, config.paths.root);
        assert_eq!(parsed.search.default_limit, config.search.default_limit);
    }
}
