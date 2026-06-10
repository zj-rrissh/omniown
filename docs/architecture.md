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
                                    ├── 启动时 spawn omniown watch (文件夹监听)
                                    └── Rust CLI 集成 (child_process)
                                          │
                                          │ child_process.spawn / exec
                                          ▼
                                    Rust CLI (src/)
                                         ├── extract (文本提取)
                                         ├── process (文件导入管线)
                                         ├── watch (文件夹监听 + 自动导入)
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
| omniown watch | Node.js 启动时自动 spawn | 递归监听 library 目录，文件增删自动同步数据库 |

## 数据流

```
用户打开桌面应用
  ↓
Tauri 壳启动 → spawn Node.js API (port 3001)
  ↓
WebView 加载 Vue 前端 → HTTP API 调用
  ↓
Prisma → SQLite (FTS5 + documents 表)
  ↓ (自动索引)
Node.js spawn omniown watch → notify 递归监听 library
  ↓ 文件增删检测
新增：index_file_in_place → 不移动文件，原地索引
删除：handle_remove → 按存储路径删除 DB 记录
  ↓
数据库与 library 目录实时同步
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

配置内容：`[ai]` (base_url, model, api_key) + `[paths]` (root, library)。

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
├── src/                  # Rust CLI (四个子命令)
│   ├── extractor.rs
│   ├── processor.rs
│   ├── mcp.rs
│   ├── watch.rs                  # 文件夹监听
│   └── main.rs                   # CLI 入口
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

**当前状态：✅ 已实现。** `omniown watch` 子命令基于 `notify` crate 递归监听 library 目录。Node.js 启动时自动 spawn watch 进程。文件放入 library 后自动索引（原地分析，不移动）；文件从 library 删除后自动清理 DB 记录。数据库通过 `--db-path` / `DATABASE_URL` 与 Node.js Prisma 共享同一个 SQLite 文件；开发环境使用 `dev.db`，桌面端使用 `omniown.db`。

**已实现：**
- ✅ `omniown watch` 子命令 — 递归监听 library 目录
- ✅ 文件新增 → 自动索引（extract + classify + upsert，原地操作）
- ✅ 文件删除 → 自动清理 DB 记录
- ✅ Node.js 服务启动时 spawn `omniown watch` 进程
- ✅ 配置变更后 Node.js 重启 → watch 自动重启
- ✅ 临时文件过滤（.tmp / .crdownload / ~$ / 隐藏文件）
- ✅ 文件稳定性检测（1s 无变化 + 大小不变才处理）
- ✅ 初始扫描 library 已有文件

### 目标 3：可自由选择 library 目录

**当前状态：✅ 已实现。** 设置页面提供 `library` 路径配置字段，并展示配置文件、数据库、知识库目录的实际位置。Node.js 启动时从配置读取路径并通过 CLI args 传给 `omniown watch`。`library` 支持绝对路径和相对路径。已移除 inbox 概念，用户直接将文件放入 library 目录即可自动索引。
