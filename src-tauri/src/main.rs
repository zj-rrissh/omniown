// 在 Windows 上隐藏控制台窗口（release 模式下生效）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::api::process::{Command as SidecarCommand, CommandChild, CommandEvent};
use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu};
use tauri_plugin_positioner::{on_tray_event, Position, WindowExt};

/// 托管 sidecar 子进程，供退出时 kill 用
struct SidecarState {
    child: Mutex<Option<CommandChild>>,
}

fn main() {
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("show_hide", "显示/隐藏"))
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", "退出"));

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .system_tray(system_tray)
        .manage(SidecarState {
            child: Mutex::new(None),
        })
        .on_system_tray_event(|app, event| {
            on_tray_event(app, &event);

            match event {
                SystemTrayEvent::LeftClick { .. }
                | SystemTrayEvent::DoubleClick { .. } => toggle_panel(app),

                SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                    "show_hide" => toggle_panel(app),
                    "quit" => {
                        // 退出前杀掉 sidecar，避免孤儿进程占用端口
                        if let Some(state) = app.try_state::<SidecarState>() {
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
            match SidecarCommand::new_sidecar("omniown").args(["serve"]) {
                Ok(cmd) => match cmd.spawn() {
                    Ok((mut rx, child)) => {
                        // 保存子进程句柄
                        let state = app.state::<SidecarState>();
                        *state.child.lock().unwrap() = Some(child);

                        // 后台线程：监听 sidecar 退出 → 自动重启
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
                                    // 自动重启
                                    match SidecarCommand::new_sidecar("omniown").args(["serve"]) {
                                        Ok(cmd) => match cmd.spawn() {
                                            Ok((new_rx, new_child)) => {
                                                let state =
                                                    app_handle.state::<SidecarState>();
                                                *state.child.lock().unwrap() = Some(new_child);
                                                rx = new_rx;
                                                continue;
                                            }
                                            Err(e) => {
                                                eprintln!(
                                                    "[sidecar] restart failed: {e}"
                                                );
                                            }
                                        },
                                        Err(e) => {
                                            eprintln!("[sidecar] restart failed: {e}");
                                        }
                                    }
                                    break; // 重启失败，退出监控
                                }
                                _ => {}
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("[sidecar] spawn failed: {e}");
                    }
                },
                Err(e) => {
                    eprintln!("[sidecar] binary not found: {e}");
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动 OmniOwn 失败");
}

/// 切换悬浮面板：已显示则隐藏，已隐藏则定位到托盘上方并显示
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
