# OmniOwn

[![CI](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml/badge.svg)](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml)

**本地优先、隐私优先的个人文档知识库。**

三层架构：Rust CLI（文本提取 + 文件管线 + MCP）→ Node.js API（Prisma + Express + FTS5 搜索）→ Vue 3 前端 + Tauri v2 桌面壳。

---

## 架构

```
Tauri Desktop (src-tauri/)                    Web 开发模式
├── WebView → ui/dist/                        npm --prefix server run dev  (port 3001)
├── spawn node server/dist/index.js           npm --prefix ui run dev      (port 5173)
├── sidecar → omniown process/extract/mcp
└── 系统托盘 + 悬浮面板
```

| 层 | 技术 | 职责 |
|:---|:---|:---|
| Rust CLI (`src/`) | rusqlite + extractor + tokio | 文本提取(PDF/DOCX/XLSX)、文件导入、MCP Server |
| Node.js API (`server/`) | Express 5 + Prisma 5 | REST API、FTS5 全文搜索、AI 多策略搜索 |
| Vue 前端 (`ui/`) | Vue 3 + Pinia + Vite 6 | 搜索/文档/配置/状态 四标签 UI |
| Tauri 桌面 (`src-tauri/`) | Tauri v2 + tray-icon | 桌面壳：托盘、悬浮面板、sidecar 管理 |

---

## 快速开始

### Web 开发模式

```bash
# 终端 1：启动 Node.js API
npm --prefix server run dev       # http://127.0.0.1:3001

# 终端 2：启动 Vue 前端
npm --prefix ui run dev           # http://127.0.0.1:5173
```

### Rust CLI

```bash
cargo build

# 导入文件
cargo run -- process inbox/note.md

# 提取文本
cargo run -- extract document.pdf

# MCP Server（供 AI 客户端调用）
cargo run -- mcp

# 生成配置模板
cargo run -- config-example
```

### Tauri 桌面

```bash
# 1. 构建 server + ui
npm --prefix server run build
npm --prefix ui run build

# 2. 打包桌面应用
cargo tauri build --manifest-path src-tauri/Cargo.toml
# → src-tauri/target/release/bundle/
```

---

## 项目结构

```
omniown/
├── src/                       # Rust CLI（7 文件）
│   ├── main.rs                # CLI 入口：process/extract/mcp
│   ├── extractor.rs           # 文本提取（PDF/DOCX/XLSX/MD/HTML/代码）
│   ├── processor.rs           # 文件导入管线（分类 + 存储 + DB 写入）
│   ├── mcp.rs                 # MCP Server（SQLite FTS5 直连）
│   ├── db.rs                  # SQLite CRUD + FTS5 + schema 初始化
│   ├── config.rs              # TOML 配置加载
│   └── fs_layout.rs           # 目录规划
│
├── server/                    # Node.js/TS API
│   ├── src/api/               # 4 路由：status/documents/search/config
│   ├── src/services/          # search / ai / import 服务
│   ├── src/db/                # Prisma client + FTS5 初始化
│   ├── src/middleware/        # 错误处理 + 请求日志
│   └── prisma/schema.prisma   # 数据库 Schema
│
├── ui/                        # Vue 3 + TypeScript 前端
│   └── src/
│       ├── views/             # Search / Documents / Config / Status
│       ├── stores/            # Pinia：documents + search
│       └── services/          # HTTP client + 按领域分片
│
├── src-tauri/                 # Tauri v2 桌面壳
│   ├── src/main.rs            # 托盘 + 面板 + Node.js 子进程
│   ├── tauri.conf.json        # 窗口/CSP/打包配置
│   └── binaries/              # Rust sidecar
│
├── docs/                      # 架构 / CLI / 配置 / 数据库 / 迁移计划
├── index/                     # Rust SQLite DB (omniown.db)
├── library/{public,private}/  # 已导入文档
├── inbox/                     # 文件监控导入目录
└── config.example.toml        # 配置模板
```

---

## 质量检查

```bash
# Rust CLI
cargo test                           # 172 tests
cargo clippy -- -D warnings          # 零警告

# Tauri 桌面
cargo test --manifest-path src-tauri/Cargo.toml  # 13 tests

# Node.js API
npm --prefix server run build        # tsc

# Vue 前端
npm --prefix ui run build            # vue-tsc + vite
```

---

## 技术栈

| 组件 | 选型 |
|:---|:---|
| 语言 | Rust (edition 2024) + TypeScript (strict) |
| 数据库 | SQLite via rusqlite (bundled) + Prisma |
| 全文检索 | FTS5 (虚拟表 + 触发器) |
| Web 前端 | Vue 3 + Pinia + Vite 6 |
| API 框架 | Express 5 |
| ORM | Prisma 5 |
| 桌面壳 | Tauri v2 (tray + shell + positioner) |
| 配置 | TOML (@iarna/toml + serde) |
| CI | GitHub Actions |
