# Changelog

All notable changes to OmniOwn are documented in this file.

---

## [0.1.0] — Unreleased

### Added

#### Desktop App (Tauri v1 + Sidecar)

- **System tray panel** — 无主窗口启动，托盘左键弹出 400×600 无边框悬浮面板，失焦自动隐藏
- **Sidecar lifecycle** — `omniown serve` 自动启停，崩溃自动重启，退出时清理子进程
- **LLM settings** — `ConfigView` 配置 API base URL / model / API key，持久化到 `config/omniown.toml`，保存后自动重启 sidecar 生效
- **MCP management** — `StatusView` 展示 4 个 MCP 工具（search_documents / get_document / list_documents / get_status），一键复制 Claude Desktop 配置
- **Tab navigation** — 底部四标签：🔍 搜索 / 📁 文档 / ⚙️ 设置 / 📊 状态
- **Document browsing** — `DocumentsView` 分页（20条/页）+ 过滤（全部/公开/私有）+ 详情浮层
- **Cross-platform** — Windows / macOS / Linux，透明无边框窗口，托盘吸附定位，Wayland fallback
- **Platform icons** — PNG (32/128/256px) + `.ico` (32px+16px) + `.icns` (128px)
- **CI/CD** — GitHub Actions 三平台构建（`.dmg` / `.exe` / `.AppImage`），sidecar 按 target-triple 注入

#### Backend

- Document import with content extraction and FTS5 indexing
- Full-text search API (`/api/search`)
- Document list API (`/api/documents`) with pagination
- Status API (`/api/status`) with document statistics
- MCP server with 4 tools for AI client integration
- AI-powered natural language search (`cargo run -- ai-search`)

#### Removed

- Embedding worker (`worker.rs`) — unused feature, removed from UI and API
- Semantic search (`/api/embedding-status`, `/api/search/semantic`) — superseded by FTS5

### Fixed

- `OmniOwnConfig` missing `#[derive(Default)]` causing compile failure

---

## [0.0.0] — Initial

- CLI tool: `cargo run` (watcher mode), `cargo run -- serve`, `cargo run -- mcp`
- Web UI: Vue 3 + TypeScript frontend served by actix-web
- SQLite database with schema versioning
- Document import pipeline (inbox → processed → library)
