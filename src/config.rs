use crate::embedding::EmbeddingProviderKind;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ---- PathsConfig ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    #[serde(default = "default_root")]
    pub root: PathBuf,
    #[serde(default = "default_inbox")]
    pub inbox: PathBuf,
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
fn default_inbox() -> PathBuf {
    PathBuf::from("inbox")
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
            inbox: default_inbox(),
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
        if let Ok(env_root) = std::env::var("OMNIOWN_ROOT") {
            self.root = PathBuf::from(env_root);
        }
        let root = self.root.clone();
        self.inbox = resolve_against(&root, &self.inbox);
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

// ---- EmbeddingConfig ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_provider")]
    pub provider: EmbeddingProviderKind,
    #[serde(default = "default_dim")]
    pub dim: usize,
    #[serde(default = "default_max_chars")]
    pub max_chars_per_doc: usize,
}

fn default_provider() -> EmbeddingProviderKind {
    EmbeddingProviderKind::Mock
}
fn default_dim() -> usize {
    384
}
fn default_max_chars() -> usize {
    100_000
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            dim: default_dim(),
            max_chars_per_doc: default_max_chars(),
        }
    }
}

// ---- WorkerConfig ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    #[serde(default = "default_worker_enabled")]
    pub enabled: bool,
    #[serde(default = "default_idle_interval_ms")]
    pub idle_interval_ms: u64,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_max_docs_per_cycle")]
    pub max_docs_per_cycle: usize,
}

fn default_worker_enabled() -> bool {
    true
}
fn default_idle_interval_ms() -> u64 {
    60_000
}
fn default_batch_size() -> usize {
    4
}
fn default_max_docs_per_cycle() -> usize {
    100
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            enabled: default_worker_enabled(),
            idle_interval_ms: default_idle_interval_ms(),
            batch_size: default_batch_size(),
            max_docs_per_cycle: default_max_docs_per_cycle(),
        }
    }
}

// ---- SearchConfig ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default = "default_search_limit")]
    pub default_limit: usize,
    #[serde(default = "default_fts_enabled")]
    pub fts_enabled: bool,
    #[serde(default = "default_semantic_enabled")]
    pub semantic_enabled: bool,
}

fn default_search_limit() -> usize {
    20
}
fn default_fts_enabled() -> bool {
    true
}
fn default_semantic_enabled() -> bool {
    true
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_limit: default_search_limit(),
            fts_enabled: default_fts_enabled(),
            semantic_enabled: default_semantic_enabled(),
        }
    }
}

// ---- AppConfig ----

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    #[serde(default)]
    pub worker: WorkerConfig,
    #[serde(default)]
    pub search: SearchConfig,
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
        if let Ok(val) = std::env::var("OMNIOWN_EMBEDDING_PROVIDER") {
            match EmbeddingProviderKind::parse(&val) {
                Ok(kind) => self.embedding.provider = kind,
                Err(e) => eprintln!(
                    "\u{26a0}\u{fe0f}  无效的 OMNIOWN_EMBEDDING_PROVIDER='{val}': {e}, 使用默认 '{}'",
                    self.embedding.provider.as_str()
                ),
            }
        }
        if let Ok(val) = std::env::var("OMNIOWN_EMBEDDING_DIM")
            && let Ok(dim) = val.parse::<usize>()
        {
            self.embedding.dim = dim.clamp(8, 4096);
        }
        if let Ok(val) = std::env::var("OMNIOWN_WORKER_ENABLED") {
            self.worker.enabled = parse_env_bool(&val);
        }
        if let Ok(val) = std::env::var("OMNIOWN_WORKER_BATCH_SIZE")
            && let Ok(bs) = val.parse::<usize>()
        {
            self.worker.batch_size = bs.clamp(1, 128);
        }
        if let Ok(val) = std::env::var("OMNIOWN_WORKER_IDLE_INTERVAL_MS")
            && let Ok(ms) = val.parse::<u64>()
        {
            self.worker.idle_interval_ms = ms.max(5000);
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

fn parse_env_bool(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "off" | "no"
    )
}

// ---- tests ----

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::{Mutex, MutexGuard};

    const ENV_KEYS: &[&str] = &[
        "OMNIOWN_ROOT",
        "OMNIOWN_DB_PATH",
        "OMNIOWN_EMBEDDING_PROVIDER",
        "OMNIOWN_EMBEDDING_DIM",
        "OMNIOWN_WORKER_ENABLED",
        "OMNIOWN_WORKER_BATCH_SIZE",
        "OMNIOWN_WORKER_IDLE_INTERVAL_MS",
    ];

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
        assert_eq!(config.embedding.provider, EmbeddingProviderKind::Mock);
        assert_eq!(config.embedding.dim, 384);
        assert!(config.worker.enabled);
        assert_eq!(config.worker.idle_interval_ms, 60_000);
        assert_eq!(config.worker.batch_size, 4);
        assert_eq!(config.search.default_limit, 20);
        assert!(config.search.fts_enabled);
        assert!(config.search.semantic_enabled);
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

[embedding]
provider = "local"
dim = 768

[worker]
enabled = false
idle_interval_ms = 30000
batch_size = 8

[search]
default_limit = 50
fts_enabled = false
"#;
        let config_path = dir.join("omniown.toml");
        let mut f = fs::File::create(&config_path).unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        let config = AppConfig::load(&dir);
        assert_eq!(config.paths.root, PathBuf::from("/tmp/test_root"));
        assert_eq!(config.embedding.provider, EmbeddingProviderKind::Local);
        assert_eq!(config.embedding.dim, 768);
        assert!(!config.worker.enabled);
        assert_eq!(config.worker.idle_interval_ms, 30_000);
        assert_eq!(config.worker.batch_size, 8);
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
        assert_eq!(config.embedding.dim, 384);
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
    fn config_env_overrides_provider() {
        let _env = EnvGuard::new();
        let mut config = AppConfig::default();
        unsafe { std::env::set_var("OMNIOWN_EMBEDDING_PROVIDER", "local") };
        config.apply_env_overrides();
        assert_eq!(config.embedding.provider, EmbeddingProviderKind::Local);
        unsafe { std::env::remove_var("OMNIOWN_EMBEDDING_PROVIDER") };
    }

    #[test]
    fn config_env_invalid_provider_returns_error() {
        let _env = EnvGuard::new();
        let mut config = AppConfig::default();
        assert_eq!(config.embedding.provider, EmbeddingProviderKind::Mock);
        unsafe { std::env::set_var("OMNIOWN_EMBEDDING_PROVIDER", "openai") };
        config.apply_env_overrides();
        assert_eq!(config.embedding.provider, EmbeddingProviderKind::Mock);
        unsafe { std::env::remove_var("OMNIOWN_EMBEDDING_PROVIDER") };
    }

    #[test]
    fn config_env_overrides_dim() {
        let _env = EnvGuard::new();
        let mut config = AppConfig::default();
        unsafe { std::env::set_var("OMNIOWN_EMBEDDING_DIM", "512") };
        config.apply_env_overrides();
        assert_eq!(config.embedding.dim, 512);
        unsafe { std::env::remove_var("OMNIOWN_EMBEDDING_DIM") };
    }

    #[test]
    fn config_dim_clamped() {
        let _env = EnvGuard::new();
        let mut config = AppConfig::default();
        unsafe { std::env::set_var("OMNIOWN_EMBEDDING_DIM", "1") };
        config.apply_env_overrides();
        assert_eq!(config.embedding.dim, 8);
        unsafe { std::env::remove_var("OMNIOWN_EMBEDDING_DIM") };
    }

    #[test]
    fn config_paths_resolve_relative() {
        let _env = EnvGuard::new();
        let paths = PathsConfig::default().resolve();
        // When root="." and no OMNIOWN_ROOT, paths are "./inbox", "./library", etc.
        assert_eq!(paths.inbox, PathBuf::from("./inbox"));
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
        assert_eq!(resolved.inbox, PathBuf::from("/data/inbox"));
        assert_eq!(resolved.library, PathBuf::from("/data/library"));
        assert_eq!(resolved.database, PathBuf::from("/data/index/omniown.db"));
    }

    #[test]
    fn config_embedding_provider_toml_deserialize() {
        let toml_mock = r#"provider = "mock""#;
        let cfg: EmbeddingConfig = toml::from_str(toml_mock).unwrap();
        assert_eq!(cfg.provider, EmbeddingProviderKind::Mock);

        let toml_local = r#"provider = "local""#;
        let cfg: EmbeddingConfig = toml::from_str(toml_local).unwrap();
        assert_eq!(cfg.provider, EmbeddingProviderKind::Local);

        let toml_upper = r#"provider = "MOCK""#;
        let cfg: EmbeddingConfig = toml::from_str(toml_upper).unwrap();
        assert_eq!(cfg.provider, EmbeddingProviderKind::Mock);
    }

    #[test]
    fn config_embedding_provider_toml_deserialize_invalid() {
        let toml_invalid = r#"provider = "openai""#;
        let result: Result<EmbeddingConfig, _> = toml::from_str(toml_invalid);
        assert!(result.is_err());
    }

    #[test]
    fn config_example_roundtrip() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.embedding.dim, config.embedding.dim);
        assert_eq!(parsed.embedding.provider, config.embedding.provider);
        assert_eq!(parsed.worker.enabled, config.worker.enabled);
        assert_eq!(parsed.worker.batch_size, config.worker.batch_size);
    }
}
