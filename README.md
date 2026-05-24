# OmniOwn

[![CI](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml/badge.svg)](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml)

**本地优先、隐私优先的个人文档知识库。**

OmniOwn 是一个 Rust CLI 本地文档管理工具 + Tauri 桌面应用。导入文本类文件，自动提取正文与元数据，建立 FTS5 全文索引，支持 AI 驱动搜索（LLM → 搜索词 → FTS5）。提供 HTTP API + MCP Server，可被 AI 客户端直接调用。

> 当前版本：**v0.1.0-alpha** — 桌面应用 Phase 1-6 已完成，待发布。

---

## 核心能力

- **文件监听** — 监控 `inbox` 目录，Create / Modify / Remove 事件自动处理
- **自动导入** — 文本类文件导入后按规则存入 `library/{public|private}/`
- **文本提取** — 统一 extractor 管线，支持纯文本、Markdown、HTML、代码、JSON/YAML/TOML/CSV
- **Hash 去重** — SHA256 内容哈希，内容未变则跳过
- **FTS5 全文搜索** — SQLite FTS5 虚拟表，实时同步，支持 snippet
- **AI 智能搜索** — `cargo run -- ai-search "问题"` → LLM 生成搜索词 → FTS5
- **MCP Server** — 4 个工具供 AI 客户端调用（search_documents / get_document / list_documents / get_status）
- **LLM 配置** — TOML 配置文件管理 API base URL / model / key
- **Schema Migration** — 数据库版本管理，幂等可重复执行
- **本地 Web UI** — Vue 3 + TypeScript 前端，`serve` 命令提供静态托管与 JSON API
- **Tauri 桌面壳** — 系统托盘 + 悬浮面板 + 四标签导航（搜索/文档/设置/状态）

## 废弃的功能

- **Embedding / 语义搜索** — 已移除。由 `ai-search` (LLM→FTS5) 替代
- **Embedding Worker** — 已移除

---

## 快速开始

### 构建与测试

```bash
cargo build
cargo test           # 265 tests
```

### 启动本地服务

```bash
# 生产模式
cd ui && npm install && npm run build && cd ..
cargo run -- serve

# 开发模式（两个终端）
# T1: cargo run -- serve
# T2: cd ui && npm install && npm run dev
```

打开 `http://127.0.0.1:17777` 浏览文档、搜索、查看状态。

### 启动哨兵（文件监控）

```bash
cargo run
```

程序监控 `./inbox` 目录，自动导入新文件。

支持的导入扩展名：`txt, md, markdown, html, htm, rs, js, ts, jsx, tsx, py, java, go, cpp, c, h, hpp, css, sh, sql, json, toml, yaml, yml, csv, log`

### 搜索

```bash
cargo run -- search rust
cargo run -- search "async queue"
```

### AI 搜索

```bash
# 先配置 AI
cargo run -- config-example   # 生成 config/omniown.toml 模板
# 编辑 config/omniown.toml，填写 [ai] 节的 api_key

cargo run -- ai-search "rust async queue 的最佳实践"
```

### 系统检查

```bash
cargo run -- doctor
cargo run -- status
```

### MCP Server

```bash
cargo run -- mcp
# 在 AI 客户端（Claude Desktop / Cursor）配置中指向此命令
```

---

## 项目结构

```
omniown/
├── src/                          # Rust 后端
│   ├── main.rs                   # CLI 入口 + 哨兵主循环
│   ├── config.rs                 # TOML 配置加载
│   ├── db.rs                     # SQLite CRUD / FTS5
│   ├── migration.rs              # Schema 迁移
│   ├── extractor.rs              # 文本提取
│   ├── classifier.rs             # 文本分类
│   ├── doctor.rs                 # 系统检查
│   ├── fs_layout.rs              # 目录规划
│   ├── processor.rs              # 文件处理管线
│   ├── storage.rs                # 文件存储路径
│   ├── ui_server.rs              # HTTP API + 静态文件
│   ├── ai.rs                     # AI 搜索（LLM→FTS5）
│   ├── mcp.rs                    # MCP Server
│   └── tests.rs                  # 集成测试
├── ui/                           # Vue 3 + TypeScript 前端
├── src-tauri/                    # Tauri v1 桌面壳
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/main.rs               # 托盘 + 面板 + sidecar
│   ├── icons/                    # 应用图标 (png/ico/icns)
│   └── binaries/                 # sidecar 目录
├── tests-config/                 # 独立 config 测试（无 Tauri 依赖）
├── scripts/                      # 辅助脚本
├── docs/                         # 文档
├── config.example.toml           # 配置模板
└── .github/workflows/            # CI (test + release)
```

---

## 技术栈

| 组件 | 选型 |
|------|------|
| 语言 | Rust (edition 2024) |
| 异步 | Tokio |
| 文件监控 | notify |
| 数据库 | SQLite via rusqlite (bundled) |
| 全文检索 | FTS5 |
| AI 搜索 | reqwest + LLM API |
| 配置 | TOML + 环境变量 |
| 序列化 | serde |
| 哈希 | SHA256 |
| Web 前端 | Vue 3 + TypeScript + Vite |
| 桌面壳 | Tauri v1 (system-tray + positioner) |
| CI | GitHub Actions |
