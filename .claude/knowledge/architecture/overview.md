# OmniOwn 架构概览

## 三层架构

```
┌─────────────────────────────────────────┐
│ Tauri v2 Shell (Rust)                   │
│ • 窗口管理（透明无边框 400×600）         │
│ • 系统托盘（显示/隐藏/退出）             │
│ • Node.js 进程生命周期（自动重启 5 次）  │
│ • MCP 进程管理（用户手动启停）           │
│ • 配置读写（TOML, app_config_dir）       │
└──────────────┬──────────────────────────┘
               │ spawn node
┌──────────────▼──────────────────────────┐
│ Node.js API (Express 5, 端口 3001)      │
│ • DB 初始化（prisma db push + WAL）      │
│ • FTS5 全文索引（虚拟表 + 3 触发器）     │
│ • omniown watch 进程管理                 │
│ • 6 个 REST API 端点                    │
│ • AI 搜索服务（LLM 策略选择）            │
└──────┬───────────────────┬──────────────┘
       │ Prisma ORM        │ spawn omniown watch
┌──────▼──────────┐  ┌─────▼─────────────────┐
│ SQLite + FTS5   │  │ Rust CLI (omniown)    │
│ • 文档表        │  │ • 文件夹监听 (notify) │
│ • FTS5 虚拟表   │  │ • 文本提取 (10+ 格式) │
│ • WAL 模式      │  │ • SHA256 去重         │
└─────────────────┘  │ • 管道式自动索引      │
                     └───────────────────────┘
```

## 进程间通信

| 发起方 | 目标 | 方式 | 数据 |
|------|------|------|------|
| Tauri | Node.js | env vars (DATABASE_URL, OMNIOWN_CONFIG_PATH) | 启动参数 |
| Node.js | Rust CLI | child_process.spawn + CLI args (--db-path, --library) | 配置路径 |
| Vue 前端 | Node.js API | HTTP (fetch, 127.0.0.1:3001) | JSON |
| Vue 前端 | Tauri | IPC (@tauri-apps/api) | Tauri commands |
| Rust CLI | SQLite | rusqlite (bundled) | 直接读写 |

## 核心数据流

```
文件 → library 目录
  → notify 事件 → 稳定性检测 (1s)
  → extractor::extract_text → 纯文本
  → processor::classify → category/domain/type
  → SHA256(file_content) → fileHash
  → db::upsert_document → documents 表
  → FTS5 触发器 → documents_fts 虚拟表
  → Node.js API 查询 → 前端展示
```

## 关键设计决策

| 决策 | 理由 | ADR |
|------|------|-----|
| SQLite + WAL 并发 | Prisma (Node) 和 rusqlite (Rust) 同时访问同一 DB | [001](./adr/001-sqlite-wal-concurrency.md) |
| 移除 inbox 概念 | 简化架构：文件直接放 library → 原地索引 | [002](./adr/002-remove-inbox.md) |
| Node.js 作为中间层 | Rust CLI 不直接暴露 HTTP；Node.js 管理进程+路由 | [003](./adr/003-nodejs-middleware.md) |
| Prisma ORM + rusqlite 共存 | Prisma 处理 API CRUD，rusqlite 处理 CLI 端实时写入 | [004](./adr/004-dual-db-access.md) |
