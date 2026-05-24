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

// ---- Tauri 命令 ----

#[tauri::command]
fn read_config(state: tauri::State<AppState>) -> Result<AiConfig, String> {
    match std::fs::read_to_string(&state.config_path) {
        Ok(content) => {
            let config: OmniOwnConfig =
                toml::from_str(&content).unwrap_or_default();
            Ok(config.ai)
        }
        Err(_) => Ok(AiConfig::default()),
    }
}

#[tauri::command]
fn write_config(
    state: tauri::State<AppState>,
    ai_config: AiConfig,
) -> Result<(), String> {
    // 读现有文件（保留非 [ai] 节）
    let mut config: toml::Value = std::fs::read_to_string(&state.config_path)
        .ok()
        .and_then(|c| toml::from_str(&c).ok())
        .unwrap_or(toml::Value::Table(Default::default()));

    // 合并 [ai] 节
    let ai_toml = toml::to_string_pretty(&ai_config).map_err(|e| e.to_string())?;
    let ai_value: toml::Value = toml::from_str(&ai_toml).map_err(|e| e.to_string())?;
    config
        .as_table_mut()
        .ok_or("invalid config")?
        .insert("ai".to_string(), ai_value);

    // 写回
    let output = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
    if let Some(parent) = state.config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&state.config_path, output).map_err(|e| e.to_string())?;

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
