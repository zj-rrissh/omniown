# 架构文档

## 总体架构

```
Tauri 桌面壳 (src-tauri/)
  ├── 系统托盘 + 悬浮面板
  ├── 启动时 spawn Node.js 服务 (sidecar)
  ├── 启动时 spawn MCP 二进制 (sidecar，按需启停)
  └── WebView 渲染 Vue 前端
             │
        ┌────┘
        ▼
Vue 3 + TS (ui/)  ─── HTTP ─── Node.js/TS API (server/)
     前端                         后端
                                    ├── Express 路由层
                                    ├── Prisma ORM + SQLite
                                    ├── FTS5 全文搜索
                                    ├── LLM 智能搜索
                                    └── Rust CLI 集成 (child_process)
                                          │
                                          │ child_process.exec
                                          ▼
                                    Rust CLI (src/)
                                         ├── extract (文本提取)
                                         ├── process (文件导入管线)
                                         └── mcp (MCP Server)
```

## 技术栈

| 层 | 技术 |
|:---|------|
| 桌面壳 | Tauri v2 (tray + shell + dialog + positioner) |
| 前端 | Vue 3 + TypeScript + Vite + Pinia |
| 后端 | Node.js + Express + TypeScript |
| 数据库 | SQLite + Prisma ORM v5 + FTS5 |
| AI 搜索 | LLM → 策略选择 → FTS5 |
| 核心处理 | Rust CLI (child_process.exec) |

## 进程模型

Tauri 启动后 spawn 两个子进程：

| 进程 | 启动时机 | 说明 |
|:---|:---|:---|
| Node.js API 服务 | Tauri setup() 阶段 | Express 服务 port 3001，自动重启（5 次/指数退避） |
| MCP 二进制 | 用户手动触发 toggle_mcp | Rust CLI 的 mcp 子命令，Tauri sidecar 方式启动 |

## 数据流

```
用户打开桌面应用
  ↓
Tauri 壳启动 → spawn Node.js API (port 3001)
  ↓
WebView 加载 Vue 前端 → HTTP API 调用
  ↓
Prisma → SQLite (FTS5 + documents 表)
  ↓ (文件导入时)
Node.js exec("omniown process <file>") → Rust CLI
  ↓
extractor → processor (classify + store + db::upsert)
```

## API 路由

| 方法 | 路径 | 说明 |
|:---|------|:-----|
| GET | `/api/status` | 系统状态（文档统计） |
| GET | `/api/documents` | 文档列表（不含 content，最近 20 条） |
| GET | `/api/documents/:id` | 文档详情（含 content） |
| GET | `/api/search?q=` | FTS5 全文搜索 |
| GET | `/api/search?q=&ai=true` | AI 多策略搜索 |
| GET | `/api/config` | 读取配置（api_key 脱敏） |
| PUT | `/api/config` | 更新配置（触发 sidecar 重启） |

## 配置管理

| 运行时 | 配置文件路径 | 说明 |
|:---|:---|:---|
| Tauri 桌面端 | `{app_config_dir}/omniown.toml` | 用户数据目录下，可持久化 |
| Node.js 独立运行 | `<server_root>/omniown.toml` | 开发/测试用 |

配置内容：`[ai]` (base_url, model, api_key) + `[paths]` (root, inbox, library)。

用户通过设置页面修改路径和 AI 配置。配置变更后 Tauri 杀旧 sidecar 子进程，由自动重启机制恢复。

## 搜索架构

```
用户输入 "我上周的代码文件"
      ↓
AI 搜索 (ai.search.ts)
      ↓
LLM → [{ strategy: "recent", params: { days: "7" } },
        { strategy: "category", params: { keyword: "code" } }]
      ↓
并行执行策略 (search.service.ts)
      ↓
合并去重 → top 20 → 返回
```

8 个搜索策略：`fulltext` / `category` / `filetype` / `summary` / `recent` / `privacy` / `filename` / `tag`

## 目录结构

```
omniown/
├── server/               # Node.js/TS API
│   ├── src/
│   │   ├── index.ts             # Express 入口 + DB 初始化
│   │   ├── api/                 # 路由层 (HTTP 请求/响应)
│   │   ├── services/            # 业务逻辑层 (搜索/导入/AI)
│   │   ├── db/                  # Prisma 客户端 + FTS5 初始化
│   │   ├── config/              # TOML 配置读取/写入
│   │   └── middleware/          # 错误处理
│   └── prisma/                  # Schema
├── ui/                   # Vue 3 + TypeScript 前端
│   └── src/
│       ├── views/               # 4 个页面 (Search/Documents/Config/Status)
│       ├── services/            # API 客户端 + 配置服务
│       ├── stores/              # Pinia 状态管理
│       └── router.ts
├── src/                  # Rust CLI (三个子命令)
│   ├── extractor.rs
│   ├── processor.rs
│   ├── mcp.rs
│   └── main.rs                  # CLI 入口
├── src-tauri/            # Tauri v2 桌面壳
│   ├── src/main.rs              # 壳逻辑 + sidecar 管理
│   ├── capabilities/            # 权限声明
│   ├── tauri.conf.json          # Tauri 配置
│   └── Cargo.toml
└── docs/                 # 项目文档
```

---

## 最终目标

### 目标 1：后端可正常读取 server/prisma/dev.db 中数据

**当前状态：✅ 已实现。** Express API 通过 Prisma 读写 SQLite，6 个端点覆盖文档 CRUD、搜索、配置。`prisma db push --skip-generate` 首次启动自动建表，FTS5 虚拟表由 `setup-fts.ts` 初始化。

### 目标 2：Rust CLI 随项目启动而启动，文件夹监听功能正常

**当前状态：⚠️ 未实现。** Rust CLI 目前仅有 `process`（单文件导入）、`extract`（文本提取）、`mcp`、`config-example` 四个子命令。**没有** `watch` 子命令，**没有** 文件夹监听功能。导入文件需手动调 API 触发 `child_process.exec("omniown process <path>")`。

**需实现：**
- `omniown watch` 子命令 — 基于 `notify` crate 监听目录
- 监听 `inbox` 目录的新增文件变化，自动触发 `process`
- Node.js 服务启动时 spawn `omniown watch` 进程
- 配置变更时重启 watch 进程

### 目标 3：可自由选择 inbox 和 library 目录

**当前状态：🔄 部分实现。** 设置页面已提供 `root`、`inbox`、`library` 三个路径配置字段，并支持系统目录选择器。配置已持久化到 TOML。但 **Rust CLI 尚未使用这些路径运行** — `omniown process` 使用自己的 `omniown.toml` 配置，与 Node.js 端配置不一定同步。

**需实现：**
- Rust CLI `watch` 和 `process` 从统一配置读取路径
- 路径支持绝对路径和相对路径（相对于 `root`）
- 配置变更后 watch 进程自动使用新路径
