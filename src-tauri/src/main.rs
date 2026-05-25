// 在 Windows 上隐藏控制台窗口（release 模式下生效）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::api::process::{Command as SidecarCommand, CommandChild, CommandEvent};
use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu};
use tauri_plugin_positioner::{on_tray_event, Position, WindowExt};

// ---- Config 数据结构 ----

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
        Self {
            base_url: String::new(),
            model: String::new(),
            api_key: String::new(),
        }
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

// ---- 托管状态 ----

/// sidecar 子进程句柄 + MCP 子进程 + 配置文件路径
struct AppState {
    child: Mutex<Option<CommandChild>>,
    mcp_child: Mutex<Option<CommandChild>>,
    config_path: PathBuf,
    mcp_running: Mutex<bool>,
}

// ---- 可测试的纯逻辑函数 ----

/// 从文件读 AiConfig；文件不存在时返回默认值
fn read_ai_config(path: &std::path::Path) -> AiConfig {
    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str::<OmniOwnConfig>(&content) {
            Ok(config) => config.ai,
            Err(e) => {
                eprintln!("[config] TOML 解析失败 {}: {e}", path.display());
                AiConfig::default()
            }
        },
        Err(_) => AiConfig::default(),
    }
}

/// 写入 AiConfig 到文件，保留其他节不变；返回写入的完整 TOML 字符串（供测试断言）
fn write_ai_config(path: &std::path::Path, ai_config: &AiConfig) -> Result<String, String> {
    // 读现有文件（保留非 [ai] 节）
    let mut config: toml::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|c| toml::from_str(&c).ok())
        .unwrap_or(toml::Value::Table(Default::default()));

    // 合并 [ai] 节
    let ai_toml = toml::to_string_pretty(ai_config).map_err(|e| e.to_string())?;
    let ai_value: toml::Value = toml::from_str(&ai_toml).map_err(|e| e.to_string())?;
    config
        .as_table_mut()
        .ok_or_else(|| "invalid config: root is not a table".to_string())?
        .insert("ai".to_string(), ai_value);

    // 写回
    let output = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, &output).map_err(|e| e.to_string())?;
    Ok(output)
}

// ---- Tauri 命令 ----

#[tauri::command]
fn read_config(state: tauri::State<AppState>) -> Result<AiConfig, String> {
    let mut config = read_ai_config(&state.config_path);
    // 不在 IPC 中明文返回完整 API key，返回脱敏版本
    if config.api_key.len() > 4 {
        let visible = &config.api_key[..4];
        config.api_key = format!("{visible}***");
    }
    Ok(config)
}

#[tauri::command]
fn read_paths_config(state: tauri::State<AppState>) -> Result<PathsSection, String> {
    match std::fs::read_to_string(&state.config_path) {
        Ok(content) => {
            let config: OmniOwnConfig = toml::from_str(&content).unwrap_or_default();
            Ok(config.paths)
        }
        Err(_) => Ok(PathsSection::default()),
    }
}

#[tauri::command]
fn write_config(
    state: tauri::State<AppState>,
    ai_config: AiConfig,
    paths_config: PathsSection,
) -> Result<(), String> {
    write_ai_config(&state.config_path, &ai_config)?;

    // 合并 [paths] 节
    let mut config: toml::Value = std::fs::read_to_string(&state.config_path)
        .ok()
        .and_then(|c| toml::from_str(&c).ok())
        .unwrap_or(toml::Value::Table(Default::default()));

    let paths_toml = toml::to_string_pretty(&paths_config).map_err(|e| e.to_string())?;
    let paths_value: toml::Value = toml::from_str(&paths_toml).map_err(|e| e.to_string())?;
    config
        .as_table_mut()
        .ok_or_else(|| "invalid config: root is not a table".to_string())?
        .insert("paths".to_string(), paths_value);

    let output = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
    if let Some(parent) = state.config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&state.config_path, output).map_err(|e| e.to_string())?;

    // 通知 sidecar 重新加载配置：杀掉 serve 和 MCP 子进程
    if let Ok(mut guard) = state.child.lock() {
        if let Some(child) = guard.take() {
            let _ = child.kill();
        }
    }
    if let Ok(mut guard) = state.mcp_child.lock() {
        if let Some(child) = guard.take() {
            let _ = child.kill();
        }
    }
    Ok(())
}

// ---- MCP 信息 ----

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpInfo {
    /// MCP 是否已启用
    ready: bool,
    /// sidecar 二进制路径
    binary: String,
    /// 可用工具列表
    tools: Vec<McpTool>,
    /// Claude Desktop 配置片段
    claude_config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpTool {
    name: &'static str,
    description: &'static str,
}

static MCP_TOOLS: &[McpTool] = &[
    McpTool {
        name: "search_documents",
        description: "Full-text search across all indexed documents using SQLite FTS5",
    },
    McpTool {
        name: "get_document",
        description: "Retrieve full content and metadata of a document by ID",
    },
    McpTool {
        name: "list_documents",
        description: "List recently updated documents with metadata",
    },
    McpTool {
        name: "get_status",
        description: "Get knowledge base statistics and index health",
    },
];

#[tauri::command]
fn mcp_info(
    state: tauri::State<AppState>,
    app_handle: tauri::AppHandle,
) -> McpInfo {
    let ready = *state.mcp_running.lock().unwrap();

    // 尝试获取 sidecar 二进制路径
    let binary = std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.parent().map(|d| {
                d.join("binaries")
                    .join(format!(
                        "omniown-{}",
                        std::env::consts::ARCH
                    ))
                    .display()
                    .to_string()
            })
        })
        .unwrap_or_else(|| "omniown".into());

    // 生成 Claude Desktop 配置
    let claude_config = format!(
        r#"{{"mcpServers":{{"omniown":{{"command":"{}","args":["mcp"]}}}}}}"#,
        binary
    );

    McpInfo {
        ready,
        binary,
        tools: MCP_TOOLS.to_vec(),
        claude_config,
    }
}

#[tauri::command]
fn toggle_mcp(state: tauri::State<AppState>) -> Result<bool, String> {
    let mut running = state.mcp_running.lock().map_err(|e| e.to_string())?;
    *running = !*running;
    let enabled = *running;

    if enabled {
        // 启动 MCP sidecar
        let cmd = match SidecarCommand::new_sidecar("omniown") {
            Ok(cmd) => cmd.args(["mcp"]),
            Err(e) => {
                *running = false;
                return Err(format!("MCP binary 未找到: {e}"));
            }
        };
        match cmd.spawn() {
            Ok((_rx, child)) => {
                *state.mcp_child.lock().map_err(|e| e.to_string())? = Some(child);
            }
            Err(e) => {
                *running = false;
                return Err(format!("MCP 启动失败: {e}"));
            }
        }
    } else {
        // 停止 MCP sidecar
        if let Ok(mut guard) = state.mcp_child.lock() {
            if let Some(child) = guard.take() {
                let _ = child.kill();
            }
        }
    }
    Ok(enabled)
}

// ---- main ----

fn main() {
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("show_hide", "显示/隐藏"))
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", "退出"));

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .system_tray(system_tray)
        .setup(|app| {
            // 使用 Tauri 路径 API 解析配置路径（而非硬编码相对路径）
            let config_path = app
                .path_resolver()
                .app_config_dir()
                .unwrap_or_else(|| PathBuf::from("../config"))
                .join("omniown.toml");

            app.manage(AppState {
                child: Mutex::new(None),
                mcp_child: Mutex::new(None),
                config_path,
                mcp_running: Mutex::new(false),
            });

            spawn_sidecar(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![read_config, read_paths_config, write_config, mcp_info, toggle_mcp])
        .on_system_tray_event(|app, event| {
            on_tray_event(app, &event);

            match event {
                SystemTrayEvent::LeftClick { .. }
                | SystemTrayEvent::DoubleClick { .. } => toggle_panel(app),

                SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                    "show_hide" => toggle_panel(app),
                    "quit" => {
                        // 清理所有子进程
                        if let Some(state) = app.try_state::<AppState>() {
                            if let Ok(mut guard) = state.child.lock() {
                                if let Some(child) = guard.take() {
                                    let _ = child.kill();
                                }
                            }
                            if let Ok(mut guard) = state.mcp_child.lock() {
                                if let Some(child) = guard.take() {
                                    let _ = child.kill();
                                }
                            }
                        }
                        std::process::exit(0);
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("启动 OmniOwn 失败");
}

fn spawn_sidecar(app: &tauri::App) {
    let cmd = match SidecarCommand::new_sidecar("omniown") {
        Ok(cmd) => cmd.args(["serve"]),
        Err(e) => {
            eprintln!("[sidecar] binary not found: {e}");
            return;
        }
    };
    match cmd.spawn() {
            Ok((mut rx, child)) => {
                let state = app.state::<AppState>();
                *state.child.lock().unwrap() = Some(child);

                let app_handle = app.handle();
                std::thread::spawn(move || {
                    let mut retries: u32 = 0;
                    const MAX_RETRIES: u32 = 5;
                    const BASE_DELAY_MS: u64 = 500;
                    loop {
                        match rx.recv() {
                            Some(CommandEvent::Error(err)) => {
                                eprintln!("[sidecar] error: {err}");
                            }
                            Some(CommandEvent::Terminated(status)) => {
                                retries += 1;
                                if retries > MAX_RETRIES {
                                    eprintln!(
                                        "[sidecar] exited (retry {retries}/{MAX_RETRIES}), giving up"
                                    );
                                    break;
                                }
                                let delay = BASE_DELAY_MS * 2u64.pow(retries - 1);
                                eprintln!(
                                    "[sidecar] exited with {:?}, restarting in {}ms (attempt {}/{})...",
                                    status.code, delay, retries, MAX_RETRIES
                                );
                                std::thread::sleep(std::time::Duration::from_millis(delay));
                                let restart_cmd = match SidecarCommand::new_sidecar("omniown") {
                                    Ok(cmd) => cmd.args(["serve"]),
                                    Err(e) => {
                                        eprintln!("[sidecar] restart failed: {e}");
                                        break;
                                    }
                                };
                                match restart_cmd.spawn() {
                                    Ok((new_rx, new_child)) => {
                                        let state = app_handle.state::<AppState>();
                                        *state.child.lock().unwrap() = Some(new_child);
                                        rx = new_rx;
                                        continue;
                                    }
                                    Err(e) => eprintln!("[sidecar] restart failed: {e}"),
                                }
                                break;
                            }
                            _ => {}
                        }
                    }
                });
            }
            Err(e) => eprintln!("[sidecar] spawn failed: {e}"),
        }
    }
}

/// 切换悬浮面板
fn toggle_panel(app: &tauri::AppHandle) {
    let window = app.get_window("main").unwrap();

    if window.is_visible().unwrap_or(false) {
        window.hide().unwrap();
    } else {
        if window.move_window(Position::TopCenter).is_err() {
            let _ = window.center();
        }
        window.show().unwrap();
        window.set_focus().unwrap();
        let _ = window.emit("tray-show", ());
    }
}

// ====================================================================
// Phase 1-3 单元测试
// ====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// 辅助：创建临时配置文件
    fn temp_config(content: &str) -> (tempfile::NamedTempFile, PathBuf) {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        let path = f.path().to_path_buf();
        (f, path)
    }

    // ---- AiConfig 序列化 ----

    #[test]
    fn ai_config_roundtrip_json() {
        let original = AiConfig {
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
            api_key: "sk-secret".into(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: AiConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.base_url, original.base_url);
        assert_eq!(restored.model, original.model);
        assert_eq!(restored.api_key, original.api_key);
    }

    #[test]
    fn ai_config_default_all_empty() {
        let cfg = AiConfig::default();
        assert!(cfg.base_url.is_empty());
        assert!(cfg.model.is_empty());
        assert!(cfg.api_key.is_empty());
    }

    #[test]
    fn ai_config_missing_fields_default_to_empty() {
        // 只有 model，没设 base_url / api_key
        let json = r#"{"model":"gpt-4"}"#;
        let cfg: AiConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.model, "gpt-4");
        assert!(cfg.base_url.is_empty());
        assert!(cfg.api_key.is_empty());
    }

    // ---- read_ai_config ----

    #[test]
    fn read_config_missing_file_returns_default() {
        let cfg = read_ai_config(std::path::Path::new("/nonexistent/omniown.toml"));
        assert!(cfg.base_url.is_empty());
        assert!(cfg.model.is_empty());
        assert!(cfg.api_key.is_empty());
    }

    #[test]
    fn read_config_empty_file_returns_default() {
        let (_f, path) = temp_config("");
        let cfg = read_ai_config(&path);
        assert!(cfg.base_url.is_empty());
        assert!(cfg.model.is_empty());
        assert!(cfg.api_key.is_empty());
    }

    #[test]
    fn read_config_parses_ai_section() {
        let (_f, path) = temp_config(
            r#"[ai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o"
api_key = "sk-abc123"
"#,
        );
        let cfg = read_ai_config(&path);
        assert_eq!(cfg.base_url, "https://api.openai.com/v1");
        assert_eq!(cfg.model, "gpt-4o");
        assert_eq!(cfg.api_key, "sk-abc123");
    }

    #[test]
    fn read_config_ignores_non_ai_sections() {
        let (_f, path) = temp_config(
            r#"[paths]
root = "/data"

[ai]
model = "claude"

[worker]
enabled = true
"#,
        );
        let cfg = read_ai_config(&path);
        assert_eq!(cfg.model, "claude");
        assert!(cfg.base_url.is_empty());
    }

    // ---- write_ai_config ----

    #[test]
    fn write_config_creates_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("omniown.toml");

        let ai = AiConfig {
            base_url: "http://localhost:11434/v1".into(),
            model: "llama3".into(),
            api_key: "".into(),
        };
        write_ai_config(&path, &ai).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("base_url = \"http://localhost:11434/v1\""));
        assert!(content.contains("model = \"llama3\""));
    }

    #[test]
    fn write_config_preserves_other_sections() {
        let original = r#"[paths]
root = "/my-data"

[worker]
enabled = false

[ai]
base_url = "old"
"#;
        let (_f, path) = temp_config(original);

        let ai = AiConfig {
            base_url: "https://new.example.com".into(),
            model: "new-model".into(),
            api_key: "new-key".into(),
        };
        write_ai_config(&path, &ai).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        // 新 ai 值写入
        assert!(content.contains("base_url = \"https://new.example.com\""));
        assert!(content.contains("model = \"new-model\""));
        // 原有 [paths] 节保留
        assert!(content.contains("[paths]"));
        assert!(content.contains("root = \"/my-data\""));
        // 原有 [worker] 节保留
        assert!(content.contains("[worker]"));
        assert!(content.contains("enabled = false"));
    }

    #[test]
    fn write_config_overwrites_existing_ai() {
        let original = r#"[ai]
base_url = "old-url"
model = "old-model"
api_key = "old-key"
"#;
        let (_f, path) = temp_config(original);

        let ai = AiConfig {
            base_url: "new-url".into(),
            model: "".into(),
            api_key: "new-key".into(),
        };
        write_ai_config(&path, &ai).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("base_url = \"new-url\""));
        assert!(content.contains("api_key = \"new-key\""));
        // model 是空串，toml 会输出空字符串
        assert!(content.contains("model = \"\""));
    }

    #[test]
    fn write_and_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("omniown.toml");

        let ai = AiConfig {
            base_url: "https://x.com/v1".into(),
            model: "m1".into(),
            api_key: "k1".into(),
        };
        write_ai_config(&path, &ai).unwrap();

        let restored = read_ai_config(&path);
        assert_eq!(restored.base_url, ai.base_url);
        assert_eq!(restored.model, ai.model);
        assert_eq!(restored.api_key, ai.api_key);
    }

    // ---- 边界条件 ----

    #[test]
    fn write_config_with_special_chars_in_api_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cfg.toml");

        let ai = AiConfig {
            base_url: "https://api.com".into(),
            model: "m1".into(),
            api_key: "sk-\"quotes\" and \\backslashes".into(),
        };
        write_ai_config(&path, &ai).unwrap();

        let restored = read_ai_config(&path);
        assert_eq!(restored.api_key, "sk-\"quotes\" and \\backslashes");
    }

    #[test]
    fn read_config_from_toml_with_array_fields_is_safe() {
        // TOML 包含数组/嵌套 — 应能被 read_ai_config 忽略
        let (_f, path) = temp_config(
            r#"[ai]
model = "safe"
[search]
tags = ["a", "b"]
[[hooks]]
name = "test"
"#,
        );
        let cfg = read_ai_config(&path);
        assert_eq!(cfg.model, "safe");
    }
}
