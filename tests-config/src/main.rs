// 独立测试 — 不依赖 Tauri
// 验证 src-tauri/src/main.rs 中 read_ai_config / write_ai_config 的纯逻辑

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

// ---- 从 main.rs 复制的纯函数 (与 Tauri 零依赖) ----

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiConfig {
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    api_key: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self { base_url: String::new(), model: String::new(), api_key: String::new() }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PathsSection {
    #[serde(default)]
    root: String,
    #[serde(default)]
    inbox: String,
    #[serde(default)]
    library: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct OmniOwnConfig {
    #[serde(default)]
    ai: AiConfig,
    #[serde(default)]
    paths: PathsSection,
}

fn read_ai_config(path: &Path) -> AiConfig {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let config: OmniOwnConfig = toml::from_str(&content).unwrap_or_default();
            config.ai
        }
        Err(_) => AiConfig::default(),
    }
}

fn write_ai_config(path: &Path, ai_config: &AiConfig) -> Result<String, String> {
    let mut config: toml::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|c| toml::from_str(&c).ok())
        .unwrap_or(toml::Value::Table(Default::default()));

    let ai_toml = toml::to_string_pretty(ai_config).map_err(|e| e.to_string())?;
    let ai_value: toml::Value = toml::from_str(&ai_toml).map_err(|e| e.to_string())?;
    config
        .as_table_mut()
        .ok_or_else(|| "invalid config: root is not a table".to_string())?
        .insert("ai".to_string(), ai_value);

    let output = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, &output).map_err(|e| e.to_string())?;
    Ok(output)
}

// ---- 测试入口 ----

fn assert_eq(a: &str, b: &str) {
    if a != b { panic!("assertion failed: {:?} != {:?}", a, b); }
}

fn assert(cond: bool, msg: &str) {
    if !cond { panic!("{}", msg); }
}

fn temp_config(content: &str) -> (tempfile::NamedTempFile, std::path::PathBuf) {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    let path = f.path().to_path_buf();
    (f, path)
}

fn main() {
    let mut passed = 0u32;
    let mut failed = 0u32;

    let mut run = |name: &str, f: fn()| {
        print!("  {:<55} ", name);
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => { println!("✅"); passed += 1; }
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() { s.clone() }
                          else if let Some(s) = e.downcast_ref::<&str>() { s.to_string() }
                          else { "panic".to_string() };
                println!("❌ {}", msg);
                failed += 1;
            }
        }
    };

    println!("\n=== AiConfig 序列化 ===");

    run("ai_config_roundtrip_json", || {
        let a = AiConfig { base_url: "https://api.openai.com/v1".into(), model: "gpt-4o-mini".into(), api_key: "sk-secret".into() };
        let b: AiConfig = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        assert_eq(a.base_url.as_str(), b.base_url.as_str());
        assert_eq(a.model.as_str(), b.model.as_str());
    });

    run("ai_config_default_all_empty", || {
        let c = AiConfig::default();
        assert(c.base_url.is_empty(), "base_url");
        assert(c.model.is_empty(), "model");
    });

    run("ai_config_missing_fields_default", || {
        let c: AiConfig = serde_json::from_str(r#"{"model":"gpt-4"}"#).unwrap();
        assert_eq(c.model.as_str(), "gpt-4");
        assert(c.base_url.is_empty(), "base_url should default");
    });

    println!("\n=== read_ai_config ===");

    run("read_missing_file_returns_default", || {
        let c = read_ai_config(Path::new("/nonexistent/cfg.toml"));
        assert(c.base_url.is_empty(), "should be empty");
    });

    run("read_empty_file_returns_default", || {
        let (_f, p) = temp_config("");
        let c = read_ai_config(&p);
        assert(c.base_url.is_empty(), "empty file");
    });

    run("read_parses_ai_section", || {
        let (_f, p) = temp_config("[ai]\nbase_url = \"https://api.openai.com/v1\"\nmodel = \"gpt-4o\"\napi_key = \"sk-abc123\"\n");
        let c = read_ai_config(&p);
        assert_eq(c.base_url.as_str(), "https://api.openai.com/v1");
        assert_eq(c.model.as_str(), "gpt-4o");
        assert_eq(c.api_key.as_str(), "sk-abc123");
    });

    run("read_ignores_non_ai_sections", || {
        let (_f, p) = temp_config("[paths]\nroot = \"/data\"\n\n[ai]\nmodel = \"claude\"\n\n[worker]\nenabled = true\n");
        let c = read_ai_config(&p);
        assert_eq(c.model.as_str(), "claude");
    });

    println!("\n=== write_ai_config ===");

    run("write_creates_new_file", || {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("cfg.toml");
        write_ai_config(&p, &AiConfig { base_url: "http://localhost:11434/v1".into(), model: "llama3".into(), api_key: "".into() }).unwrap();
        let s = std::fs::read_to_string(&p).unwrap();
        assert(s.contains("base_url = \"http://localhost:11434/v1\""), "missing base_url");
    });

    run("write_preserves_other_sections", || {
        let (_f, p) = temp_config("[paths]\nroot = \"/d\"\n\n[worker]\nenabled = false\n\n[ai]\nbase_url = \"old\"\n");
        write_ai_config(&p, &AiConfig { base_url: "https://new.example.com".into(), model: "m".into(), api_key: "k".into() }).unwrap();
        let s = std::fs::read_to_string(&p).unwrap();
        assert(s.contains("[paths]"), "lost [paths]");
        assert(s.contains("[worker]"), "lost [worker]");
        assert(s.contains("base_url = \"https://new.example.com\""), "ai not updated");
    });

    run("write_overwrites_existing_ai", || {
        let (_f, p) = temp_config("[ai]\nmodel = \"old\"\n");
        write_ai_config(&p, &AiConfig { base_url: "".into(), model: "new".into(), api_key: "".into() }).unwrap();
        assert(std::fs::read_to_string(&p).unwrap().contains("model = \"new\""), "not overwritten");
    });

    run("write_read_roundtrip", || {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("cfg.toml");
        let a = AiConfig { base_url: "https://x.com/v1".into(), model: "m1".into(), api_key: "k1".into() };
        write_ai_config(&p, &a).unwrap();
        let b = read_ai_config(&p);
        assert_eq(b.base_url.as_str(), a.base_url.as_str());
        assert_eq(b.model.as_str(), a.model.as_str());
        assert_eq(b.api_key.as_str(), a.api_key.as_str());
    });

    println!("\n=== read_paths_config ===");

    run("read_paths_missing_file_returns_default", || {
        let cfg: OmniOwnConfig = toml::from_str("").unwrap_or_default();
        assert(cfg.paths.root.is_empty(), "root");
        assert(cfg.paths.inbox.is_empty(), "inbox");
        assert(cfg.paths.library.is_empty(), "library");
    });

    run("read_paths_parses_section", || {
        let (_f, p) = temp_config("[paths]\nroot = \"/data\"\ninbox = \"/home/user/inbox\"\nlibrary = \"/mnt/lib\"\n");
        let content = std::fs::read_to_string(&p).unwrap();
        let cfg: OmniOwnConfig = toml::from_str(&content).unwrap_or_default();
        assert_eq(cfg.paths.root.as_str(), "/data");
        assert_eq(cfg.paths.inbox.as_str(), "/home/user/inbox");
        assert_eq(cfg.paths.library.as_str(), "/mnt/lib");
    });

    run("write_preserves_paths_section", || {
        let (_f, p) = temp_config("[paths]\nroot = \"/my-root\"\ninbox = \"/my-inbox\"\n[ai]\nmodel = \"m1\"\n");
        let ai = AiConfig { base_url: "https://new.com".into(), model: "m2".into(), api_key: "".into() };
        write_ai_config(&p, &ai).unwrap();
        let content = std::fs::read_to_string(&p).unwrap();
        assert(content.contains("root = \"/my-root\""), "paths.root lost");
        assert(content.contains("inbox = \"/my-inbox\""), "paths.inbox lost");
        assert(content.contains("model = \"m2\""), "ai updated");
    });

    run("ai_and_paths_sections_coexist", || {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("cfg.toml");
        let content = "[ai]\nmodel = \"gpt\"\n\n[paths]\nroot = \"/r\"\ninbox = \"/i\"\nlibrary = \"/l\"\n";
        std::fs::write(&p, content).unwrap();
        let raw = std::fs::read_to_string(&p).unwrap();
        let cfg: OmniOwnConfig = toml::from_str(&raw).unwrap_or_default();
        assert_eq(cfg.ai.model.as_str(), "gpt");
        assert_eq(cfg.paths.root.as_str(), "/r");
        assert_eq(cfg.paths.inbox.as_str(), "/i");
        assert_eq(cfg.paths.library.as_str(), "/l");
    });

    println!("\n=== 边界条件 ===");

    run("special_chars_in_api_key", || {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("cfg.toml");
        let a = AiConfig { base_url: "https://api.com".into(), model: "m1".into(), api_key: "sk-\"quotes\" and \\backslashes".into() };
        write_ai_config(&p, &a).unwrap();
        let b = read_ai_config(&p);
        assert_eq(b.api_key.as_str(), "sk-\"quotes\" and \\backslashes");
    });

    run("toml_with_array_fields_is_safe", || {
        let (_f, p) = temp_config("[ai]\nmodel = \"safe\"\n[search]\ntags = [\"a\", \"b\"]\n[[hooks]]\nname = \"test\"\n");
        let c = read_ai_config(&p);
        assert_eq(c.model.as_str(), "safe");
    });

    println!("\n═══════════════════════════════════════");
    println!("  结果: {} passed, {} failed", passed, failed);
    println!("═══════════════════════════════════════\n");

    if failed > 0 { std::process::exit(1); }
}
