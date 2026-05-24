# OmniOwn 桌面端开发计划

> 将 OmniOwn 从 CLI 工具 + Web 服务转变为**桌面应用**（Tauri v1 + sidecar），用户下载即用。

---

## 1. 目标

- 用户下载安装包后开箱即用，无需装 Rust/Node.js
- 启动仅显示系统托盘图标，无主窗口
- 左键托盘图标 → 右下方弹出悬浮面板；再点隐藏
- 点击面板外部区域 → 面板自动隐藏
- 提供配置界面设置 LLM API key/model
- 内置 MCP server 开关，支持 AI 客户端连接

---

## 2. 架构方案：Tauri v1 + Sidecar

```
┌───────────────────────────────────────────────────┐
│                  Tauri v1 进程                       │
│                                                     │
│  ┌──────────────────────────────────────────────┐  │
│  │  main.rs                                      │  │
│  │  · 系统托盘 (SystemTray)                       │  │
│  │  · 左键单击 / 右键菜单                          │  │
│  │  · setup: 隐藏窗口，仅留托盘图标                 │  │
│  └──────────────┬───────────────────────────────┘  │
│                 │ show / hide                       │
│  ┌──────────────▼───────────────────────────────┐  │
│  │  WebView 悬浮面板 (400×600)                   │  │
│  │  · decorations: false (无边框)                │  │
│  │  · transparent: true (背景透明)               │  │
│  │  · alwaysOnTop: true (置顶)                   │  │
│  │  · visible: false (启动隐藏)                  │  │
│  │  · ← tauri-plugin-positioner                 │  │
│  │    Position::TrayCenter (吸附托盘上方)          │  │
│  │  · ← 前端 onFocusChanged(false) → hide()     │  │
│  └──────────────────────────────────────────────┘  │
│                                                     │
│  sidecar 管理 (Command::new_sidecar)                │
│  ┌──────────────────────────────────────────────┐  │
│  │  omniown serve (127.0.0.1:17777)             │  │
│  │  · HTTP API                                  │  │
│  │  · MCP Server (按需)                         │  │
│  └──────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────┘
```

**为什么 sidecar + 托盘面板：**
- 现有 Rust 后端零修改，HTTP API 和 MCP 直接复用
- 托盘面板模式与 Windows 日历、Wi-Fi 面板体验一致
- 未来可独立更新后端或前端

---

## 3. 项目结构

```
omniown/
├── src/                          # 现有 Rust 后端（不变）
│   └── ...
│
├── ui/                           # Vue 前端（修改）
│   ├── src/
│   │   ├── api.ts                # ✅ 移除 embedding 字段，保留 FTS 搜索 API
│   │   ├── App.vue               # ✅ 失焦隐藏 + 拖拽 + Tauri 事件监听
│   │   └── ...
│   └── package.json              # ✅ 加 @tauri-apps/api@^1
│
├── src-tauri/                    # ✅ Tauri v1 项目
│   ├── Cargo.toml
│   ├── build.rs                  # tauri_build::build()
│   ├── tauri.conf.json
│   ├── src/
│   │   └── main.rs               # 托盘 + 面板 toggle
│   ├── icons/                    # 应用 / 托盘图标 (png / ico / icns)
│   └── binaries/                 # sidecar 目录（Phase 2 使用）
│
├── docs/
│   └── desktop-plan.md           # 本文档
│
└── .gitignore                    # ✅ 加 /src-tauri/target
```

> **注意：** `src-tauri/` 是**独立的 Rust 项目**，不与根 `Cargo.toml` 共享 workspace。

---

## 4. 分阶段实施

### ✅ Phase 1：Tauri v1 托盘面板（已完成）

**实现：**

| 文件 | 内容 |
|------|------|
| `src-tauri/Cargo.toml` | Tauri v1 + system-tray + positioner + shell |
| `src-tauri/build.rs` | `tauri_build::build()` |
| `src-tauri/tauri.conf.json` | 无边框/透明/置顶/隐藏 + 托盘配置 + macOSPrivateApi |
| `src-tauri/src/main.rs` | 系统托盘 + LeftClick/DoubleClick toggle + Position::TrayCenter + Wayland fallback + tray-show 事件 |
| `src-tauri/icons/` | 32/128/256px PNG + .ico (双层) + .icns |
| `ui/src/api.ts` | 移除 embeddings/worker/embedding_status/SemanticSearchResult/semanticSearch |
| `ui/src/App.vue` | `data-tauri-drag-region` 拖拽 + `onFocusChanged` 失焦隐藏 + `tray-show` 事件防竞态 |
| `ui/package.json` | `@tauri-apps/api@^1.6.0` |
| `.gitignore` | `/src-tauri/target` |

**关键设计决策：**

| 决策 | 理由 |
|------|------|
| `LeftClick \| DoubleClick` 都 toggle | Windows 有些场景触发 DoubleClick 而非 LeftClick |
| Rust emit `tray-show` + 前端 500ms 防抖 | 解决托盘 click 与失焦隐藏竞态 |
| `move_window` 失败时 `window.center()` | Wayland 不支持客户端定位 |
| `npm --prefix ../ui run build` | 跨平台（替代 Unix-only 的 `cd ../ui &&`） |

**平台支持：**

| 平台 | 托盘定位 | 无边框透明 | 系统依赖 |
|------|:---:|:---:|------|
| Windows 10/11 | ✅ Taskbar | ✅ DWM 自带 | 无（WebView2 内置于 OS） |
| macOS | ✅ Menu Bar | ✅ + macOSPrivateApi | 无 |
| Linux X11 | ✅ System Tray | ✅ compositor | `libwebkit2gtk-4.0-dev libsoup2.4-dev pkg-config` |
| Linux Wayland | ⚠️ fallback center | ✅ compositor | 同上 |

---

### ✅ Phase 2：sidecar 集成（已完成）

**目标：** Tauri 启动时自动拉起 omniown HTTP server，退出时清理，崩溃自动重启

**实现：**

| 文件 | 内容 |
|------|------|
| `src-tauri/src/main.rs` | `setup` 中 `Command::new_sidecar("omniown").args(["serve"]).spawn()`；`SidecarState` Mutex 保存 `CommandChild`；quit 时 `.kill()` 清理；std::thread 监听 `CommandEvent::Terminated` → 自动重启 |
| `src-tauri/Cargo.toml` | `shell-sidecar` feature（已有） |
| `src-tauri/binaries/omniown-x86_64-unknown-linux-gnu` | sidecar 二进制（已构建并放置） |
| `scripts/build-sidecar.sh` | 自动化：构建 omniown → 复制到 binaries/ 加 target-triple 后缀 |

**生命周期：**
```
Tauri 启动 → setup → spawn sidecar → 后台线程监听
                  ↓                          │
            存储 CommandChild          ┌──────┘
                  ↓                   │ 崩溃
             托盘 quit               ▼
                  ↓              自动重启
           kill sidecar
                  ↓
         std::process::exit(0)
```

| 步骤 | 内容 | 文件 | 状态 |
|------|------|------|:--:|
| 2.1 | `externalBin` + shell scope | `tauri.conf.json` | ✅ |
| 2.2 | `setup` 中 spawn sidecar `["serve"]` | `main.rs` | ✅ |
| 2.3 | quit 时 `child.kill()` 清理 | `main.rs` | ✅ |
| 2.4 | `CommandEvent::Terminated` → 自动重启 | `main.rs` | ✅ |
| 2.5 | `scripts/build-sidecar.sh` 构建辅助脚本 | `scripts/` | ✅ |

> **注意：** Tauri v1 sidecar 要求二进制文件名为 `{name}-{target_triple}` 格式（如 `omniown-x86_64-unknown-linux-gnu`）。`externalBin` 中写不带后缀的名字，Tauri 自动匹配。

---

### ✅ Phase 3：LLM 设置界面（已完成）

**实现：**

| 文件 | 内容 |
|------|------|
| `src-tauri/Cargo.toml` | 新增 `serde = { features = ["derive"] }`、`toml = "0.8"` |
| `src-tauri/src/main.rs` | `#[tauri::command] read_config` → 读 `config/omniown.toml` 返回 AiConfig；`write_config` → 合并 [ai] 节写回 + 杀掉 sidecar 触发重启 |
| `ui/src/views/ConfigView.vue` | 表单：API base URL / model / API key，调用 `invoke('read_config'/'write_config')` |
| `ui/src/views/StatusView.vue` | 调用 `/api/status` 展示文档/数据库/Schema 统计 |
| `ui/src/views/SearchView.vue` | 搜索/文档浏览功能（从 App.vue 抽出） |
| `ui/src/router.ts` | Hash 路由：`/` → SearchView、`/config` → ConfigView、`/status` → StatusView |
| `ui/src/App.vue` | 壳组件：Tauri 事件监听 + `<router-view/>` + 底部三标签导航 |
| `ui/src/main.ts` | 启用 `createApp(App).use(router)` |
| `ui/package.json` | `vue-router@4` |

**架构：**
```
App.vue (壳)
├── Tauri 事件 (tray-show / onFocusChanged)
├── <router-view>
│   ├── /         → SearchView   (搜索 + 文档列表 + 详情)
│   ├── /config   → ConfigView   (LLM API 设置)
│   └── /status   → StatusView   (系统统计)
└── 底部导航栏 (搜索 / 设置 / 状态)
```

**配置持久化流程：**
```
用户填写 API key → invoke('write_config', { aiConfig })
  → Rust 读取 ../config/omniown.toml
  → 合并 [ai] 节，保留其他节不变
  → 写回文件
  → kill sidecar → 监控线程自动 restart
  → sidecar 重读配置 → 生效
```

---

### ✅ Phase 4：MCP 管理（已完成）

**目标：** 查看 MCP 工具列表，获取 AI 客户端配置，一键开关

**设计决策：** MCP 使用 stdio 协议，由 AI 客户端（Claude Desktop / Cursor）主动启动 `omniown mcp` 子进程。Tauri 应用不托管 MCP 进程，而是提供配置信息。

**实现：**

| 文件 | 内容 |
|------|------|
| `src-tauri/src/main.rs` | `McpInfo` 结构体；`mcp_info` 命令返回 tools 列表 + binary 路径 + Claude 配置片段；`toggle_mcp` 切换 `mcp_running` 标志 |
| `ui/src/views/StatusView.vue` | MCP 专区：开关按钮 + 4 工具列表 + 可复制的 AI 客户端配置 |

**MCP 工具 (4 个)：**

| 工具 | 用途 |
|------|------|
| `search_documents` | FTS5 全库搜索 |
| `get_document` | 按 ID 获取文档内容 |
| `list_documents` | 近期文档列表（可过滤 public/private） |
| `get_status` | 知识库统计信息 |

---

### Phase 5：UI 适配 + 路由（~2h）

**目标：** 完整的桌面导航体验

| 步骤 | 内容 | 文件 |
|------|------|------|
| 5.1 | 增加 Vue Router：搜索 / 文档列表 / 配置 / 状态 | `ui/src/router.ts` |
| 5.2 | 搜索页：搜索框 + 结果列表 + 点击查看详情 | `ui/src/views/SearchView.vue` |
| 5.3 | 文档列表页：分页 + 过滤 | `ui/src/views/DocumentsView.vue` |
| 5.4 | 状态页：文档统计、MCP 状态 | `ui/src/views/StatusView.vue` |

---

### Phase 6：打包与发布（~4h）

**目标：** CI 自动构建 .dmg / .exe / .AppImage

| 步骤 | 内容 | 文件 |
|------|------|------|
| 6.1 | `cargo tauri icon` 生成多尺寸应用图标 | `src-tauri/icons/` |
| 6.2 | GitHub Actions + `tauri-action` 三平台构建 | `.github/workflows/release.yml` |
| 6.3 | sidecar 按 target-triple 注入 | CI 脚本 |
| 6.4 | CHANGELOG 更新 | `CHANGELOG.md` |

---

## 5. 开发环境搭建

### 各平台前置依赖

| 平台 | 依赖安装命令 |
|------|-------------|
| **Windows** | 无（WebView2 内置于 Win10/11） |
| **macOS** | 无（WebKit 内置于 OS） |
| **Linux (Debian/Ubuntu)** | `sudo apt install -y libwebkit2gtk-4.0-dev libsoup2.4-dev pkg-config` |
| **Linux (Fedora)** | `sudo dnf install -y webkit2gtk4.0-devel libsoup2-devel pkgconf` |

### Tauri CLI

```bash
cargo install tauri-cli --version "^1"
```

### 启动开发

```bash
# 终端 1: 启动后端
cargo run -- serve

# 终端 2: 启动 Tauri 桌面壳
cargo tauri dev
```

### WSL + Windows 混合开发

WSL 和 Windows 共享 `127.0.0.1`，可以 WSL 跑后端 + Windows 跑 Tauri 壳：

```
┌── WSL ──────────┐     ┌── Windows ────────────┐
│ cargo serve     │     │ cargo tauri dev       │
│ :17777          │◀───│ WebView → :17777       │
└─────────────────┘     └───────────────────────┘
```

> 从 Windows 访问 WSL 项目：`cd \\wsl$\Ubuntu\home\<user>\workspace\omniown\src-tauri`
>
> 如果编译慢，把项目 `git clone` 到 `D:\Projects\omniown`，Tauri 壳在 Windows 本地编译更快。

---

## 6. 已知问题与平台差异

| 问题 | 平台 | 缓解 |
|------|------|------|
| `move_window` 在 Wayland 无效 | Linux | fallback `window.center()` |
| 透明窗口需要 `macOSPrivateApi: true` | macOS | `tauri.conf.json` 中已开启 |
| 失焦隐藏与托盘 click 竞态 | Windows/Linux | Rust emit `tray-show` + 前端 500ms 防抖 |
| 多实例 | 全部 | 后续加 `tauri-plugin-single-instance` |
| sidecar 二进制需 `{name}-{triple}` 命名 | 全部 | CI 构建时自动重命名 |
| Windows 托盘图标可能被折叠 | Windows | 用户拖拽到可见区（系统行为） |
| Linux 需安装 WebKit 开发库 | Linux | 见上方依赖安装命令 |

---

## 7. 发布版本规划

| 版本 | 内容 | 目标 |
|------|------|------|
| v0.1.0-alpha | Phase 1 + 2（托盘面板 + sidecar） | 内部测试 |
| v0.1.0-beta | Phase 3 + 4（LLM 配置 + MCP） | 公开测试 |
| v0.1.0 | Phase 5 + 6（完整 UI + CI 构建） | 首次发布 |

---

## 8. 工作量汇总

| Phase | 内容 | 状态 | 预估 |
|:-----|------|:----:|:----:|
| 1 | Tauri 托盘面板 | ✅ | ~5h |
| 2 | sidecar 集成 | ✅ | ~2h |
| 3 | LLM 配置界面 | ✅ | ~3h |
| 4 | MCP 管理 | ✅ | ~2h |
| 5 | UI 路由适配 | ⬜ | ~2h |
| 6 | 打包发布 CI | ⬜ | ~4h |
| **合计** | | **67%** | **~18h** |
