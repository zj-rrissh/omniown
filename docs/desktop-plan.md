# OmniOwn 桌面端开发计划

> 将 OmniOwn 从 CLI 工具 + Web 服务转变为**桌面应用**（Tauri + sidecar），用户下载即用。

---

## 1. 目标

- 用户下载安装包后开箱即用，无需装 Rust/Node.js
- 桌面托盘常驻后台，自动监控 inbox 导入文件
- 提供配置界面设置 LLM API key/model
- 内置 MCP server 开关，支持 AI 客户端连接
- 系统开机自启动（可选）

---

## 2. 架构方案：Tauri + Sidecar

```
┌──────────────────────────────────────────────────┐
│                  Tauri 进程                        │
│                                                    │
│  ┌──────────────┐     ┌────────────────────────┐  │
│  │  Rust 壳层    │────▶│ 系统托盘 menu           │  │
│  │  (tauri)      │     │  · 显示/隐藏窗口         │  │
│  │               │     │  · 开机自启开关          │  │
│  │  sidecar 管理  │     │  · MCP 开关             │  │
│  │  起/停/监控    │     │  · 退出                  │  │
│  └──────┬───────┘     └────────────────────────┘  │
│         │                                          │
│         │ 启动 / 停止                               │
│         ▼                                          │
│  ┌──────────────────────────────────────┐          │
│  │  omniown sidecar (现有 Rust binary)   │          │
│  │                                      │          │
│  │  · 哨兵模式 (inbox 监控)              │          │
│  │  · HTTP API (127.0.0.1:17777)        │          │
│  │  · MCP Server (stdio)                │          │
│  └──────────────────────────────────────┘          │
│                                                    │
│  ┌──────────────────────────────────────┐          │
│  │  WebView (Vue 前端)                   │          │
│  │                                      │          │
│  │  · 文档浏览 / 搜索                    │          │
│  │  · LLM 配置页                        │          │
│  │  · MCP 配置页                        │          │
│  │  · 系统状态面板                      │          │
│  │  ← → HTTP API (localhost)           │          │
│  └──────────────────────────────────────┘          │
└──────────────────────────────────────────────────┘
```

**为什么选 sidecar：**
- 现有 Rust 后端零修改，HTTP API 和 MCP 直接复用
- Vue 前端只改 API 地址（从相对路径到 `http://127.0.0.1:17777`）
- 未来可独立更新后端或前端

---

## 3. 项目结构变化

```
omniown/
├── src/                          # 现有 Rust 后端（不变）
│   ├── main.rs                   # CLI + sentinel（保留）
│   ├── mcp.rs                    # MCP server（保留）
│   ├── ui_server.rs              # HTTP API（保留）
│   └── ...
│
├── ui/                           # 现有 Vue 前端（修改）
│   ├── src/
│   │   ├── api.ts                # 更新：移除 embedding 字段
│   │   ├── App.vue               # 更新：适配桌面窗口
│   │   ├── views/
│   │   │   ├── SearchView.vue    # 🔄 搜索页
│   │   │   ├── DocumentsView.vue # 🔄 文档列表
│   │   │   ├── ConfigView.vue    # 🆕 LLM + MCP 设置
│   │   │   └── StatusView.vue    # 🆕 系统状态
│   │   └── main.ts
│   ├── package.json
│   └── vite.config.ts            # 更新：build target
│
├── src-tauri/                    # 🆕 Tauri 项目
│   ├── Cargo.toml                # Tauri 壳依赖
│   ├── tauri.conf.json           # 窗口、托盘、sidecar 配置
│   ├── src/
│   │   ├── main.rs               # Tauri 入口 + 命令
│   │   ├── lib.rs                # sidecar 管理
│   │   └── tray.rs               # 系统托盘
│   ├── icons/                    # 应用图标
│   └── binaries/                 # sidecar 二进制（构建时注入）
│
├── .github/workflows/
│   ├── ci.yml                    # 现有 CI（不变）
│   └── release.yml               # 🆕 Tauri 构建 + 发布
│
├── docs/
│   ├── desktop-plan.md           # 本文档
│   └── architecture.md           # 🔄 更新架构图
│
└── ROADMAP.md                    # 🔄 更新
```

---

## 4. 分阶段实施

### Phase 1：基础搭建（~4h）

**目标：** Tauri 项目跑起来，WebView 显示 Vue UI

| 步骤 | 内容 | 文件 |
|------|------|------|
| 1.1 | 初始化 Tauri v2 项目 | `src-tauri/` 目录 |
| 1.2 | 配置 sidecar：注册 `omniown` binary | `tauri.conf.json` → `bundle.externalBin` |
| 1.3 | Tauri 壳启动时自动拉起 sidecar（`Command::new_sidecar`） | `src-tauri/src/lib.rs` |
| 1.4 | Tauri 壳关闭时杀掉 sidecar | `src-tauri/src/lib.rs` |
| 1.5 | 配置 WebView 指向 sidecar 的 HTTP 地址 | `tauri.conf.json` → `app.url` |
| 1.6 | Vue 前端 `api.ts` 改用绝对 URL：`http://127.0.0.1:17777` | `ui/src/api.ts` |
| 1.7 | 移除 `api.ts` 中已删除的 embedding/worker 字段 | `ui/src/api.ts` |

**验收：** `cargo tauri dev` 启动后显示 Vue 界面，API 正常返回

### Phase 2：系统托盘（~3h）

**目标：** 托盘常驻、背景运行、开机自启

| 步骤 | 内容 | 文件 |
|------|------|------|
| 2.1 | 注册系统托盘菜单（显示/隐藏/退出/开关） | `src-tauri/src/tray.rs` |
| 2.2 | 托盘图标（active + inactive 状态） | `src-tauri/icons/` |
| 2.3 | Tauri 命令：`toggle_window`、`quit_app` | `src-tauri/src/lib.rs` |
| 2.4 | 开机自启动配置（Tauri `auto-launch` plugin） | `Cargo.toml` + `lib.rs` |
| 2.5 | 窗口关闭时隐藏到托盘而非退出 | `tauri.conf.json` |
| 2.6 | sidecar 异常退出时自动重启 | `src-tauri/src/lib.rs` |

**验收：** 关闭窗口 → 托盘图标 → 右键菜单可唤出/退出

### Phase 3：LLM 设置界面（~3h）

**目标：** 用户可在 UI 中配置 LLM API key/model，配置持久化到文件

| 步骤 | 内容 | 文件 |
|------|------|------|
| 3.1 | Tauri 命令：`read_config` / `write_config` | `src-tauri/src/lib.rs` |
| 3.2 | 配置界面 Vue 组件：API base URL、model、API key | `ui/src/views/ConfigView.vue` |
| 3.3 | 配置持化为 `config/omniown.toml` | 调用 `write_config` Tauri 命令 |
| 3.4 | 导航栏增加"设置"入口 | `ui/src/App.vue` |
| 3.5 | 状态显示：当前 LLM 配置是否有效 | `ui/src/views/StatusView.vue` |

**验收：** 在 UI 中填 LLM key → 保存 → 生效 → `ai-search` 可用

### Phase 4：MCP 管理（~2h）

**目标：** 一键开启/关闭 MCP server，显示状态

| 步骤 | 内容 | 文件 |
|------|------|------|
| 4.1 | sidecar 增加 `mcp` 子命令启动（已有） | 已有 |
| 4.2 | Tauri 命令：`toggle_mcp` 通过 sidecar stdin 发送启停信号 | `src-tauri/src/lib.rs` |
| 4.3 | MCP 状态指示器（运行中/已停止） | `ui/src/views/StatusView.vue` |
| 4.4 | 托盘增加 MCP 开关 | `src-tauri/src/tray.rs` |

**验收：** 点击开关 → MCP 启动/停止 → AI 客户端可/不可连接

### Phase 5：文档适配 + 路由（~2h）

**目标：** UI 完整的桌面导航体验

| 步骤 | 内容 | 文件 |
|------|------|------|
| 5.1 | 增加 Vue Router（搜索 / 文档列表 / 配置 / 状态） | `ui/src/router.ts` 🆕 |
| 5.2 | 搜索页：搜索框 + 结果列表 + 点击查看详情 | `ui/src/views/SearchView.vue` 🆕 |
| 5.3 | 文档列表页：分页列表 + 过滤 | `ui/src/views/DocumentsView.vue` 🆕 |
| 5.4 | 状态页：文档统计、schema 版本、MCP 状态 | `ui/src/views/StatusView.vue` 🆕 |
| 5.5 | 底部导航栏（移动端适配） | `ui/src/App.vue` |

**验收：** 页面间导航流畅，搜索/查看/配置均可操作

### Phase 6：打包与发布（~4h）

**目标：** CI 自动构建 .dmg / .exe / .AppImage，发布 GitHub Release

| 步骤 | 内容 | 文件 |
|------|------|------|
| 6.1 | Tauri 构建配置（名称、图标、版本、证书） | `tauri.conf.json` |
| 6.2 | 应用图标（1024×1024 → 各平台格式） | `src-tauri/icons/` |
| 6.3 | GitHub Actions：`tauri-action` 构建三平台 | `.github/workflows/release.yml` 🆕 |
| 6.4 | 自动上传 artifact 到 GitHub Release | `.github/workflows/release.yml` |
| 6.5 | sidecar 预编译二进制注入 | `tauri.conf.json` → `externalBin` |
| 6.6 | CHANGELOG 更新 | `CHANGELOG.md` 🆕 |
| 6.7 | 更新 ROADMAP.md | `ROADMAP.md` |

**验收：** 触发 tag push → CI 自动产出 `.dmg` / `.msi` / `.AppImage`

---

## 5. 需要新增/修改的文件清单

### 🆕 新增文件

| 文件 | 说明 |
|------|------|
| `src-tauri/Cargo.toml` | Tauri 壳依赖 |
| `src-tauri/tauri.conf.json` | 窗口、sidecar、图标配置 |
| `src-tauri/src/main.rs` | Tauri 入口 |
| `src-tauri/src/lib.rs` | 命令注册、sidecar 生命周期管理 |
| `src-tauri/src/tray.rs` | 系统托盘菜单 |
| `src-tauri/icons/` | 应用图标（多尺寸多平台） |
| `ui/src/router.ts` | Vue Router 配置 |
| `ui/src/views/SearchView.vue` | 搜索页面 |
| `ui/src/views/DocumentsView.vue` | 文档列表页面 |
| `ui/src/views/ConfigView.vue` | LLM + MCP 配置页面 |
| `ui/src/views/StatusView.vue` | 系统状态页面 |
| `.github/workflows/release.yml` | Tauri 构建发布 CI |
| `CHANGELOG.md` | 版本日志 |

### 🔄 修改文件

| 文件 | 改动 |
|------|------|
| `ui/src/api.ts` | API URL 改为 `http://127.0.0.1:17777`；移除 `embeddings`、`worker`、`embedding_status` 字段 |
| `ui/src/App.vue` | 增加导航栏/侧边栏、路由视图 |
| `ui/vite.config.ts` | 可能需要调整 build target |
| `ui/package.json` | 添加 vue-router 依赖 |
| `Cargo.toml` | 增加 `[workspace]` 成员（src-tauri） |
| `docs/architecture.md` | 更新架构图，移除 embedding |
| `ROADMAP.md` | 更新进度 |

---

## 6. 依赖

### Rust（新增）

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"          # sidecar 管理
tauri-plugin-autostart = "2"      # 开机自启
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### Node.js（新增）

```
vue-router         # 页面路由
@tauri-apps/api    # Tauri IPC 前端 SDK
```

---

## 7. 风险与应对

| 风险 | 概率 | 影响 | 应对 |
|------|------|------|------|
| Tauri v2 sidecar API 不稳定 | 低 | 中 | 锁定 Tauri 版本号，参考官方 example |
| WebView 跨域请求 sidecar API | 中 | 高 | 确保 sidecar 绑定 `127.0.0.1`，Tauri WebView 用 `devUrl` 或 `url` 指向 localhost |
| sidecar 崩溃后自动重启 | 低 | 中 | lib.rs 中监听 sidecar exit status，自动重新 spawn |
| 现有 `ui/api.ts` 前端字段与后端 API 不一致 | 高 | 中 | Phase 1 先修复所有不匹配的接口类型 |
| 三平台 sidecar 二进制管理 | 中 | 中 | Tauri sidecar 支持按平台自动选择正确二进制 |

---

## 8. 发布版本规划

| 版本 | 内容 | 目标 |
|------|------|------|
| v0.1.0-alpha | Phase 1 + 2（基础桌面 + 托盘） | 内部测试 |
| v0.1.0-beta | Phase 3 + 4（LLM 配置 + MCP 管理） | 公开测试 |
| v0.1.0 | Phase 5 + 6（完整 UI + 自动构建） | 首次发布 |

---

## 9. 工作量汇总

| Phase | 内容 | 预估时间 |
|:-----|------|:--------:|
| 1 | Tauri 基础搭建 + sidecar 集成 | ~4h |
| 2 | 系统托盘 + 后台运行 | ~3h |
| 3 | LLM 配置界面 | ~3h |
| 4 | MCP 管理 | ~2h |
| 5 | 文档适配 + 路由 | ~2h |
| 6 | 打包与发布 CI | ~4h |
| **合计** | | **~18h（3-4天）** |
