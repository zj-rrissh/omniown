// 在 Windows 上隐藏控制台窗口（release 模式下生效）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_positioner::{Position, WindowExt};
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;

// ---- Config 数据结构 ----

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AiConfig {
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    api_key: String,
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
    child: Mutex<Option<Child>>,
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
        if let Some(child) = guard.as_mut() {
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

#[derive(Debug, Clone, Serialize)]
struct McpInfo {
    ready: bool,
    binary: String,
    tools: Vec<McpTool>,
    claude_config: String,
}

#[derive(Debug, Clone, Serialize)]
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
fn mcp_info(state: tauri::State<AppState>, _app_handle: tauri::AppHandle) -> McpInfo {
    let ready = *state.mcp_running.lock().unwrap();

    let sidecar_ext = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    let binary = std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.parent().map(|d| {
                d.join("binaries")
                    .join(format!(
                        "omniown-{}{}",
                        env!("TAURI_ENV_TARGET_TRIPLE"),
                        sidecar_ext
                    ))
                    .display()
                    .to_string()
            })
        })
        .unwrap_or_else(|| "omniown".into());

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
fn toggle_mcp(state: tauri::State<AppState>, app_handle: tauri::AppHandle) -> Result<bool, String> {
    let mut running = state.mcp_running.lock().map_err(|e| e.to_string())?;
    *running = !*running;
    let enabled = *running;

    if enabled {
        let shell = app_handle.shell();
        let cmd = match shell.sidecar("omniown") {
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
        if let Ok(mut guard) = state.mcp_child.lock() {
            if let Some(child) = guard.take() {
                let _ = child.kill();
            }
        }
    }
    Ok(enabled)
}

// ---- Win32 FFI (panic 弹窗) ----

#[cfg(windows)]
mod win {
    extern "system" {
        pub fn MessageBoxW(hwnd: isize, text: *const u16, caption: *const u16, utype: u32) -> i32;
    }
}

// ---- main ----

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("{info}");
        #[cfg(windows)]
        {
            use win::MessageBoxW;
            let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
            let title: Vec<u16> = "OmniOwn 启动错误"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                MessageBoxW(0, wide.as_ptr(), title.as_ptr(), 0x10); // MB_ICONERROR
            }
        }
        #[cfg(not(windows))]
        eprintln!("{msg}");
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config_path = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| PathBuf::from("../config"))
                .join("omniown.toml");

            app.manage(AppState {
                child: Mutex::new(None),
                mcp_child: Mutex::new(None),
                config_path,
                mcp_running: Mutex::new(false),
            });

            // 系统托盘
            let show_hide = MenuItem::with_id(app, "show_hide", "显示/隐藏", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&show_hide, &PredefinedMenuItem::separator(app)?, &quit],
            )?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show_hide" => toggle_panel(app),
                    "quit" => {
                        if let Some(state) = app.try_state::<AppState>() {
                            if let Ok(mut guard) = state.child.lock() {
                                if let Some(mut child) = guard.take() {
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
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        toggle_panel(app);
                    }
                })
                .build(app)?;

            spawn_sidecar(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_config,
            read_paths_config,
            write_config,
            mcp_info,
            toggle_mcp
        ])
        .run(tauri::generate_context!())
        .expect("启动 OmniOwn 失败");
}

fn node_command_works(node_command: &Path) -> bool {
    let command = PathBuf::from(node_path_arg(node_command));
    std::process::Command::new(command)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn resolve_node_command(resource_dir: &Path) -> PathBuf {
    let mut candidates = Vec::new();

    if cfg!(target_os = "windows") {
        candidates.push(resource_dir.join("node").join("win-x64").join("node.exe"));
    }

    if let Ok(program_files) = std::env::var("ProgramFiles") {
        candidates.push(PathBuf::from(program_files).join("nodejs/node.exe"));
    }
    if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
        candidates.push(PathBuf::from(program_files_x86).join("nodejs/node.exe"));
    }

    candidates
        .into_iter()
        .find(|path| path.exists() && node_command_works(path))
        .unwrap_or_else(|| {
            let path_node = PathBuf::from("node");
            if node_command_works(&path_node) {
                path_node
            } else {
                PathBuf::from("node")
            }
        })
}

fn sidecar_file_name() -> String {
    let ext = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    format!("omniown-{}{}", env!("TAURI_ENV_TARGET_TRIPLE"), ext)
}

fn resolve_omniown_binary_path(resource_dir: &Path) -> Option<PathBuf> {
    let sidecar_name = sidecar_file_name();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|parent| parent.to_path_buf()));

    let mut candidates = Vec::new();
    if let Some(dir) = exe_dir {
        let app_exe_name = if cfg!(target_os = "windows") {
            "omniown.exe"
        } else {
            "omniown"
        };
        candidates.push(dir.join(app_exe_name));
        candidates.push(dir.join("binaries").join(&sidecar_name));
    }
    candidates.push(resource_dir.join("binaries").join(&sidecar_name));
    candidates.push(resource_dir.join(&sidecar_name));

    candidates.into_iter().find(|path| path.exists())
}

fn append_server_log(data_dir: &Path, message: &str) {
    let log_path = data_dir.join("server.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        use std::io::Write;
        let _ = writeln!(file, "{message}");
    }
}

fn node_path_arg(path: &Path) -> String {
    let value = path.display().to_string();
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
}

fn toml_path_arg(path: &Path) -> String {
    node_path_arg(path).replace('\\', "/")
}

fn default_runtime_config(default_library: &Path) -> String {
    format!(
        r#"[paths]
root = "."
library = "{}"

[search]
default_limit = 20
fts_enabled = true

[ai]
base_url = ""
model = ""
api_key = ""
"#,
        toml_path_arg(default_library)
    )
}

fn ensure_runtime_config(config_path: &Path, data_dir: &Path) -> Result<(), String> {
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let default_library = data_dir.join("library");
    std::fs::create_dir_all(&default_library).map_err(|e| e.to_string())?;

    let content = match std::fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(_) => {
            std::fs::write(config_path, default_runtime_config(&default_library))
                .map_err(|e| e.to_string())?;
            eprintln!("[config] 已创建默认配置: {}", config_path.display());
            return Ok(());
        }
    };

    let mut config: toml::Value = match toml::from_str(&content) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("[config] 配置文件解析失败，重写默认配置: {e}");
            std::fs::write(config_path, default_runtime_config(&default_library))
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
    };

    let table = config
        .as_table_mut()
        .ok_or_else(|| "invalid config: root is not a table".to_string())?;
    let paths = table
        .entry("paths".to_string())
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .ok_or_else(|| "invalid config: paths is not a table".to_string())?;

    let mut changed = false;
    let root_missing = paths
        .get("root")
        .and_then(|v| v.as_str())
        .map(|v| v.trim().is_empty())
        .unwrap_or(true);
    if root_missing {
        paths.insert("root".to_string(), toml::Value::String(".".to_string()));
        changed = true;
    }

    let library_missing = paths
        .get("library")
        .and_then(|v| v.as_str())
        .map(|v| v.trim().is_empty())
        .unwrap_or(true);
    if library_missing {
        paths.insert(
            "library".to_string(),
            toml::Value::String(toml_path_arg(&default_library)),
        );
        changed = true;
    }

    if changed {
        let output = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
        std::fs::write(config_path, output).map_err(|e| e.to_string())?;
        eprintln!("[config] 已修复默认路径配置: {}", config_path.display());
    }

    Ok(())
}

fn spawn_node_server(
    node_command: &Path,
    server_js: &Path,
    db_url: &str,
    config_path: &str,
    prisma_schema: &Path,
    omniown_bin: Option<&Path>,
    data_dir: &Path,
) -> Result<Child, String> {
    let log_path = data_dir.join("server.log");
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("无法打开后端日志 {}: {e}", log_path.display()))?;
    let stderr = stdout
        .try_clone()
        .map_err(|e| format!("无法复制后端日志句柄 {}: {e}", log_path.display()))?;

    let server_js_arg = node_path_arg(server_js);
    let prisma_schema_arg = node_path_arg(prisma_schema);
    let data_dir_arg = node_path_arg(data_dir);
    let omniown_bin_arg = omniown_bin.map(node_path_arg);

    let node_command_arg = PathBuf::from(node_path_arg(node_command));
    let mut cmd = Command::new(&node_command_arg);
    cmd.arg(&server_js_arg)
        .env("DATABASE_URL", db_url)
        .env("OMNIOWN_CONFIG_PATH", config_path)
        .env("PRISMA_SCHEMA_PATH", &prisma_schema_arg)
        .current_dir(&data_dir_arg)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    if let Some(bin) = &omniown_bin_arg {
        cmd.env("OMNIOWN_BIN", bin);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    cmd.spawn().map_err(|e| {
        format!(
            "Node.js 启动失败: {e}; command={}, entry={}",
            node_command_arg.display(),
            server_js_arg
        )
    })
}

fn spawn_sidecar(app: &tauri::App) {
    // 启动 Node.js API 服务
    let resource_dir = app
        .path()
        .resource_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    // 使用用户数据目录（可写）存放数据库，避免 Windows Program Files 权限问题
    let data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| resource_dir.clone());
    let _ = std::fs::create_dir_all(&data_dir);

    let node_command = resolve_node_command(&resource_dir);
    if !node_command_works(&node_command) {
        let msg = format!(
            "OmniOwn 后端 Node.js 运行时缺失或不可执行。\n请重新安装最新版本，或检查安装包是否包含 bundled Node runtime。\n\nNode command: {}",
            node_command.display()
        );
        append_server_log(&data_dir, &format!("[server] {msg}"));
        #[cfg(target_os = "windows")]
        {
            use win::MessageBoxW;
            let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
            let title: Vec<u16> = "Node.js 运行时不可用"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                MessageBoxW(0, wide.as_ptr(), title.as_ptr(), 0x30); // MB_ICONWARNING
            }
        }
        #[cfg(not(target_os = "windows"))]
        eprintln!("[server] {msg}");
        return;
    }

    let server_js = resource_dir.join("server/dist/index.js");
    let server_js_path = server_js.clone();
    let prisma_schema = resource_dir.join("server/dist/prisma/schema.prisma");
    let omniown_bin = resolve_omniown_binary_path(&resource_dir);
    let db_url = format!("file:{}", data_dir.join("dev.db").display());

    // 配置文件路径 — 与 Tauri read_config/write_config 使用同一文件
    let config_path = app.state::<AppState>().config_path.clone();

    // 确保 Node.js 和 Rust watch 都能读到 TOML 安全且非空的 library 路径。
    if let Err(e) = ensure_runtime_config(&config_path, &data_dir) {
        eprintln!("[config] 准备运行时配置失败: {e}");
        append_server_log(&data_dir, &format!("[config] 准备运行时配置失败: {e}"));
    }

    let config_path_str = config_path.display().to_string();
    let db_url_restart = db_url.clone();
    let data_dir_restart = data_dir.clone();
    let server_js_restart = server_js_path.clone();
    let prisma_schema_restart = prisma_schema.clone();
    let omniown_bin_restart = omniown_bin.clone();

    let node_command_display = node_path_arg(&node_command);
    eprintln!("[server] Node command: {}", node_command_display);
    eprintln!("[server] API entry: {}", server_js_path.display());
    eprintln!("[server] Prisma schema: {}", prisma_schema.display());
    if let Some(bin) = &omniown_bin {
        eprintln!("[server] OmniOwn binary: {}", bin.display());
    }

    append_server_log(
        &data_dir,
        &format!(
            "[server] starting node={}, entry={}, prisma={}, omniown={}",
            node_command_display,
            server_js_path.display(),
            prisma_schema.display(),
            omniown_bin
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<none>".into())
        ),
    );

    let child = match spawn_node_server(
        &node_command,
        &server_js_path,
        &db_url,
        &config_path_str,
        &prisma_schema,
        omniown_bin.as_deref(),
        &data_dir,
    ) {
        Ok(child) => child,
        Err(e) => {
            eprintln!("[server] {e}");
            append_server_log(&data_dir, &format!("[server] {e}"));
            eprintln!("[server] 请确认已安装 Node.js 并执行 npm --prefix server run build");
            return;
        }
    };
    let state = app.state::<AppState>();
    *state.child.lock().unwrap() = Some(child);

    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let mut retries: u32 = 0;
        const MAX_RETRIES: u32 = 5;
        const BASE_DELAY_MS: u64 = 500;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            let exited = {
                let state = app_handle.state::<AppState>();
                let mut guard = state.child.lock().unwrap();
                match guard.as_mut() {
                    Some(child) => match child.try_wait() {
                        Ok(Some(status)) => {
                            *guard = None;
                            Some(format!("{:?}", status.code()))
                        }
                        Ok(None) => None,
                        Err(e) => {
                            *guard = None;
                            Some(format!("try_wait error: {e}"))
                        }
                    },
                    None => None,
                }
            };

            if let Some(exit_detail) = exited {
                retries += 1;
                if retries > MAX_RETRIES {
                    let msg = format!(
                        "[server] exited ({exit_detail}, retry {retries}/{MAX_RETRIES}), giving up"
                    );
                    eprintln!("{msg}");
                    append_server_log(&data_dir_restart, &msg);
                    break;
                }
                let delay = BASE_DELAY_MS * 2u64.pow(retries - 1);
                let msg = format!(
                    "[server] exited with {exit_detail}, restarting in {delay}ms (attempt {retries}/{MAX_RETRIES})..."
                );
                eprintln!("{msg}");
                append_server_log(&data_dir_restart, &msg);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;

                match spawn_node_server(
                    &node_command,
                    &server_js_restart,
                    &db_url_restart,
                    &config_path_str,
                    &prisma_schema_restart,
                    omniown_bin_restart.as_deref(),
                    &data_dir_restart,
                ) {
                    Ok(new_child) => {
                        let state = app_handle.state::<AppState>();
                        *state.child.lock().unwrap() = Some(new_child);
                    }
                    Err(e) => {
                        eprintln!("[server] restart failed: {e}");
                        append_server_log(
                            &data_dir_restart,
                            &format!("[server] restart failed: {e}"),
                        );
                        break;
                    }
                }
            }
        }
    });
}

/// 切换悬浮面板
fn toggle_panel(app: &tauri::AppHandle) {
    let window = app.get_webview_window("main").unwrap();

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
        assert!(content.contains("base_url = \"https://new.example.com\""));
        assert!(content.contains("model = \"new-model\""));
        assert!(content.contains("[paths]"));
        assert!(content.contains("root = \"/my-data\""));
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
