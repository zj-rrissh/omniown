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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OmniOwnConfig {
    #[serde(default)]
    ai: AiConfig,
}

// ---- 托管状态 ----

/// sidecar 子进程句柄 + 配置文件路径
struct AppState {
    child: Mutex<Option<CommandChild>>,
    config_path: PathBuf,
}

// ---- 可测试的纯逻辑函数 ----

/// 从文件读 AiConfig；文件不存在时返回默认值
fn read_ai_config(path: &std::path::Path) -> AiConfig {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let config: OmniOwnConfig = toml::from_str(&content).unwrap_or_default();
            config.ai
        }
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
    Ok(read_ai_config(&state.config_path))
}

#[tauri::command]
fn write_config(
    state: tauri::State<AppState>,
    ai_config: AiConfig,
) -> Result<(), String> {
    write_ai_config(&state.config_path, &ai_config)?;

    // 通知 sidecar 重新加载配置：杀掉旧进程，监控线程会自动重启
    if let Ok(mut guard) = state.child.lock() {
        if let Some(child) = guard.take() {
            let _ = child.kill();
        }
    }
    Ok(())
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
        .manage(AppState {
            child: Mutex::new(None),
            config_path: PathBuf::from("../config/omniown.toml"),
        })
        .invoke_handler(tauri::generate_handler![read_config, write_config])
        .on_system_tray_event(|app, event| {
            on_tray_event(app, &event);

            match event {
                SystemTrayEvent::LeftClick { .. }
                | SystemTrayEvent::DoubleClick { .. } => toggle_panel(app),

                SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                    "show_hide" => toggle_panel(app),
                    "quit" => {
                        if let Some(state) = app.try_state::<AppState>() {
                            if let Ok(mut guard) = state.child.lock() {
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
        .setup(|app| {
            // Phase 2: 启动 sidecar（omniown serve 模式）
            spawn_sidecar(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动 OmniOwn 失败");
}

fn spawn_sidecar(app: &tauri::App) {
    match SidecarCommand::new_sidecar("omniown").args(["serve"]) {
        Ok(cmd) => match cmd.spawn() {
            Ok((mut rx, child)) => {
                let state = app.state::<AppState>();
                *state.child.lock().unwrap() = Some(child);

                let app_handle = app.handle();
                std::thread::spawn(move || loop {
                    match rx.recv() {
                        Some(CommandEvent::Error(err)) => {
                            eprintln!("[sidecar] error: {err}");
                        }
                        Some(CommandEvent::Terminated(status)) => {
                            eprintln!(
                                "[sidecar] exited with {:?}, restarting...",
                                status.code
                            );
                            match SidecarCommand::new_sidecar("omniown").args(["serve"])
                            {
                                Ok(cmd) => match cmd.spawn() {
                                    Ok((new_rx, new_child)) => {
                                        let state = app_handle.state::<AppState>();
                                        *state.child.lock().unwrap() = Some(new_child);
                                        rx = new_rx;
                                        continue;
                                    }
                                    Err(e) => eprintln!("[sidecar] restart failed: {e}"),
                                },
                                Err(e) => eprintln!("[sidecar] restart failed: {e}"),
                            }
                            break;
                        }
                        _ => {}
                    }
                });
            }
            Err(e) => eprintln!("[sidecar] spawn failed: {e}"),
        },
        Err(e) => eprintln!("[sidecar] binary not found: {e}"),
    }
}

/// 切换悬浮面板
fn toggle_panel(app: &tauri::AppHandle) {
    let window = app.get_window("main").unwrap();

    if window.is_visible().unwrap_or(false) {
        window.hide().unwrap();
    } else {
        if window.move_window(Position::TrayCenter).is_err() {
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
