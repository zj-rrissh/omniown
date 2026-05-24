// 在 Windows 上隐藏控制台窗口（release 模式下生效）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu};
use tauri_plugin_positioner::{on_tray_event, Position, WindowExt};

fn main() {
    // ---- 系统托盘菜单 ----
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("show_hide", "显示/隐藏"))
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", "退出"));

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        // 注册定位插件：让 move_window 使用托盘位置
        .plugin(tauri_plugin_positioner::init())
        // 注册系统托盘
        .system_tray(system_tray)
        // ---- 托盘事件处理 ----
        .on_system_tray_event(|app, event| {
            // positioner 记录本次托盘事件的位置（后续 move_window 使用）
            on_tray_event(app, &event);

            match event {
                // 左键单击 / 双击 → 切换面板显示
                SystemTrayEvent::LeftClick { .. }
                | SystemTrayEvent::DoubleClick { .. } => toggle_panel(app),

                // 右键菜单项
                SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                    "show_hide" => toggle_panel(app),
                    "quit" => {
                        // TODO Phase 2: kill sidecar 子进程
                        std::process::exit(0);
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        // ---- 启动时：窗口已配置 visible: false，仅显示托盘图标 ----
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("启动 OmniOwn 失败");
}

/// 切换悬浮面板：已显示则隐藏，已隐藏则定位到托盘上方并显示
fn toggle_panel(app: &tauri::AppHandle) {
    let window = app.get_window("main").unwrap();

    if window.is_visible().unwrap_or(false) {
        window.hide().unwrap();
    } else {
        // 使用 positioner 定位到托盘图标中心上方
        // Wayland 等不支持 move_window 的平台降级为居中
        if window.move_window(Position::TrayCenter).is_err() {
            let _ = window.center();
        }
        window.show().unwrap();
        window.set_focus().unwrap();

        // 通知前端：本次 show 来自托盘点击
        // 前端收到后短暂屏蔽 blur 隐藏，防止竞态
        let _ = window.emit("tray-show", ());
    }
}
