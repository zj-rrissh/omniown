# 全栈迁移回顾

> 本文档记录了 OmniOwn 从 Rust 单体重构为 Vue 3 + Node.js + Tauri 全栈应用的过程。
> 迁移已基本完成，具体实现参见 [architecture.md](./architecture.md)。

---

## 迁移成果

### 已完成

| 模块 | 原实现 | 当前实现 | 状态 |
|:---|:---|:---|:---|
| HTTP API | Rust `ui_server.rs` | Node.js Express 路由分层 | ✅ |
| 数据库 | Rust `rusqlite` | Prisma ORM v5 + SQLite | ✅ |
| Schema 迁移 | Rust migration 系统 | `prisma db push --skip-generate` | ✅ |
| 配置管理 | Rust `config.rs` | TOML (Node.js + Rust 双端读写) | ✅ |
| 文档分类 | Rust `classifier.rs` | 保留在 Rust CLI 中 | ✅ |
| AI 搜索 | Rust `ai.rs` | Node.js `ai.service.ts` (axios + OpenAI SDK) | ✅ |
| 前端 | Vue 3 + TS (直接调 Rust) | Vue 3 + TS (调 Node.js API) | ✅ |
| Tauri 桌面壳 | Rust sidecar `omniown serve` | Tauri sidecar `node server/dist/index.js` | ✅ |
| MCP Server | Rust `mcp.rs` | 保留 Rust CLI，Tauri sidecar 启停 | ✅ |
| 文件夹监听 | 无 | `omniown watch` — notify + 自动导入 | ✅ |
| CI/CD | 无 | GitHub Actions (ci + release) | ✅ |

### 保留 Rust

| 模块 | 保留原因 |
|:---|:---|
| `extractor.rs` | PDF/DOCX/XLSX 解析需 Rust 成熟库 |
| `processor.rs` | 文件管线含原子写入 + 去重 + 冲突处理 |
| `mcp.rs` | MCP 协议与 stdio 通信 |
| `main.rs` (CLI) | `process` / `extract` / `watch` / `mcp` / `config-example` |
| `watch.rs` | 文件夹监听 + 自动导入 |

---

## 架构对比

### 原架构 (Rust 单体)

```
Vue 3 + TS ── HTTP ── Rust 单体 (src/)
                        ├── ui_server.rs (HTTP API)
                        ├── db.rs (SQLite CRUD + FTS5)
                        ├── processor.rs
                        └── ...
            ┌── Tauri 壳 ── sidecar ──┘
```

### 当前架构

```
Tauri 壳 ── spawn ── Node.js API (port 3001) ── HTTP ── Vue 前端 (WebView)
                       ├── Prisma + SQLite
                       ├── FTS5 + AI 搜索
                       ├── spawn omniown watch (library 递归监听)
                       └── child_process ── Rust CLI
```

---

## 仍待实现

| 任务 | 说明 | 相关文档 |
|:---|:---|:---|
| 前端文档详情页「打开文件」按钮 | 用系统默认程序打开 library 中的文件 | 待规划 |
| Windows 端到端测试 | 在 Windows 上验证安装、启动、文件索引全流程 | 待执行 |
