# OmniOwn 全栈迁移方案

> 目标：将项目从 Rust 单体重构为 Vue 3 + TypeScript 全栈应用。
> Rust 仅保留核心重型处理（文本提取、文件管线），Node.js/TS 负责业务 API 层和前端。

---

## 架构对比

### 当前架构

```
Vue 3 + TS (ui/)  ─── HTTP ─── Rust 单体 (src/)
                                    ├── ui_server.rs (HTTP API)
                                    ├── db.rs (SQLite CRUD + FTS5)
                                    ├── processor.rs (文件管线)
                                    ├── extractor.rs (文本提取)
                                    ├── classifier.rs (分类)
                                    ├── ai.rs (LLM 调用)
                                    ├── config.rs (配置管理)
                                    ├── mcp.rs (MCP Server)
                                    └── main.rs (CLI + 哨兵)
                    ┌── Tauri 桌面壳 ── sidecar ──┘
```

### 目标架构

```
                    Tauri 桌面壳 (src-tauri/)
                    ├── 系统托盘 + 悬浮面板
                    ├── 启动时 spawn Node.js sidecar
                    └── 退出时清理子进程
                         │
                    ┌────┘
                    ▼
Vue 3 + TS (ui/)  ─── HTTP ─── Node.js/TS API (server/)
     (Tauri WebView)                 (sidecar)
                                    ├── src/api/ (路由 / 控制器)
                                    ├── src/db/ (Prisma + SQLite)
                                    ├── src/services/ (业务逻辑)
                                    ├── src/config/ (配置)
                                    ├── src/middleware/ (日志/CORS)
                                    │
                                    │ child_process
                                    │
                                    └── Rust CLI (保留核心)
                                        ├── omniown extract  (文本提取)
                                        ├── omniown process  (文件导入)
                                        ├── omniown mcp     (MCP Server)
                                        └── omniown ai-search (AI 搜索)
```

**交付形态：** Tauri 桌面壳启动时 spawn Node.js 服务作为 sidecar，Vue 前端在 Tauri WebView 中通过 HTTP 调用 Node.js API。最终用户拿到的是 `.dmg` / `.exe` / `.AppImage`。

---

## 模块替换映射

### 完全替换为 TypeScript

| Rust 文件 | 职责 | TS 替代方案 |
|-----------|------|-------------|
| `ui_server.rs` | HTTP API | `Express.js` / `Fastify` + 路由分层 |
| `db.rs` | SQLite CRUD + FTS5 | `Prisma ORM` + SQLite FTS5 raw query |
| `migration.rs` | Schema 迁移 | `Prisma Migrate` |
| `classifier.rs` | 文档分类 | TypeScript 纯函数 |
| `ai.rs` | LLM 调用 | `axios` + openai SDK |
| `config.rs` | 配置加载 | `dotenv` + JSON 配置文件 |
| `doctor.rs` | 健康检查 | Health check route |
| `storage.rs` | 路径生成 | TypeScript 纯函数 |
| `main.rs` (serve) | 服务入口 | `ts-node` / `tsx` 入口 |

### 保留 Rust (通过 CLI 调用)

| Rust 文件 | 保留原因 | 调用方式 |
|-----------|---------|---------|
| `extractor.rs` | 需解析 PDF/DOCX/XLSX，Rust 有成熟库 | `child_process.exec("omniown extract <file>")` |
| `processor.rs` | 文件管线含原子写入 | `child_process.exec("omniown process <file>")` |
| `mcp.rs` | MCP 协议与 stdio 通信 | 独立进程运行 |
| `main.rs` (core) | CLI 入口 | 编排所有 Rust 操作 |

---

## 阶段计划

### Phase A：Node.js 后端骨架（2-3 天）

```
server/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts              # 入口：Express 启动
│   ├── config/
│   │   └── index.ts          # dotenv + 配置加载
│   ├── db/
│   │   ├── client.ts          # Prisma 客户端
│   │   └── schema.prisma      # Prisma Schema
│   ├── api/
│   │   ├── status.ts          # GET /api/status
│   │   ├── search.ts          # GET /api/search
│   │   ├── documents.ts       # GET /api/documents /api/documents/:id
│   │   └── config.ts          # GET/PUT /api/config
│   ├── services/
│   │   ├── search.service.ts  # FTS5 搜索
│   │   ├── ai.service.ts      # LLM 搜索 (Rust CLI fallback)
│   │   └── import.service.ts  # Rust CLI 编排
│   └── middleware/
│       ├── error.ts           # 统一错误处理
│       └── logger.ts          # 请求日志
├── prisma/
│   └── schema.prisma          # 数据库 Schema
```

**验收标准：**
- [ ] `npm run dev` 启动 Express 开发服务器
- [ ] Prisma Migrate 初始化 SQLite 数据库
- [ ] Rust API 现有路由全部移植到 Node.js
- [ ] `/api/*` 返回与原来一致的 JSON 结构

**验收标准：**
- [ ] 所有现有路由移植到 Node.js
- [ ] `/api/*` 返回与原来一致的 JSON 结构

### Phase C：Vue 前端重构（2 天）

- 当前 Vue 代码已是 TS，但状态管理可增强

```ts
// 新增模块
stores/             # Pinia 状态管理
  ├── documents.store.ts   # 文档列表缓存
  └── search.store.ts     # 搜索状态

api.ts → services/      # 拆分为按领域分片
  ├── documents.service.ts
  └── search.service.ts
```

**验收标准：**
- [ ] Pinia 替换组件内直接 API 调用
- [ ] 前端对接 Node.js API 而非 Rust

### Phase D：Rust CLI 集成（1 天）

```ts
// src/services/import.service.ts
import { execa } from 'execa'

export async function importFile(filePath: string) {
  const { stdout } = await execa('omniown', ['process', filePath])
  return JSON.parse(stdout)
}

export async function extractText(filePath: string) {
  const { stdout } = await execa('omniown', ['extract', filePath])
  return stdout
}
```

**验收标准：**
- [ ] Node.js 通过 `child_process` / `execa` 调用 Rust CLI
- [ ] Rust CLI 输出 JSON 供 Node.js 消费
- [ ] 错误从 Rust 正确传递到 API

### Phase E：MCP Server（0.5 天）

保留 `omniown mcp` 作为独立进程，Node.js 不替代此模块。

### Phase F：桌面端打包（1 天）

```bash
# 1. 构建 Node.js API → dist/
npm --prefix server run build

# 2. 构建 Vue 前端
npm --prefix ui run build

# 3. 更新 Tauri sidecar 配置
#    src-tauri/tauri.conf.json 中 externalBin 改为指向 Node.js 服务
#    Tauri 壳启动 node server/dist/index.js 而非 omniown serve

# 4. Tauri 打包
cargo tauri build
# → src-tauri/target/release/bundle/
#   ├── OmniOwn.dmg       (macOS)
#   ├── OmniOwn.msi       (Windows)
#   └── OmniOwn.AppImage  (Linux)
```

**前提条件**
- Node.js 与 Rust 均需预装在 CI runner 上
- `tauri.conf.json` 的 `beforeBuildCommand` 需分别 build server/ 和 ui/
- GitHub Actions `release.yml` 已配置三平台构建，只需替换构建步骤

---

## API 对照表

新旧 API 签名保持一致，前端几乎不改。

| 当前 Rust API | Node.js 替换 | 改动 |
|:-------------|:------------|:----|
| `GET /api/status` | `GET /api/status` | 文档统计 |
| `GET /api/documents` | `GET /api/documents` | 分页参数用 Prisma |
| `GET /api/documents/:id` | `GET /api/documents/:id` | — |
| `GET /api/search?q=` | `GET /api/search?q=` | — |
| `GET /api/config` | `GET /api/config` | — |
| `POST /api/config` | `PUT /api/config` | — |

---

## 目录结构（最终形态）

```
omniown/
├── server/                 # ★ 新增 — Node.js/TS API
│   ├── src/
│   ├── prisma/
│   ├── package.json
│   └── tsconfig.json
│
├── ui/                     # 现有 — Vue 3 + TS 前端
│   └── src/
│       ├── services/       # ★ 重构 — 按领域分片
│       ├── stores/         # ★ 新增 — Pinia
│       └── views/          # ★ 增强 — 更丰富的 UI
│
├── src/                    # 缩减 — Rust 核心
│   ├── extractor.rs        # ✅ 保留
│   ├── processor.rs        # ✅ 保留
│   ├── mcp.rs              # ✅ 保留
│   └── main.rs             # 精简为 CLI 入口
│
├── docs/
│   └── migration-plan.md   # 本文档
│
├── src-tauri/               # 桌面壳（保持不变）
└── README.md               # 更新安装说明
```

---

## 时间预估

| Phase | 内容 | 预估 | 简历亮点 |
|:-----|------|:---:|---------|
| A | Node.js 后端骨架 | 2-3 天 | Express + Prisma + RESTful |
| B | Vue 重构 + Pinia | 2 天 | 状态管理 + 组件化 |
| C | Rust CLI 集成 | 1 天 | `child_process` / `execa` |
| D | MCP 保留 | — | MCP 协议（已实现） |
| E | 桌面端打包 | 1 天 | Tauri + GitHub Actions CI |
| **合计** | | **6-7 天** | |

---

## 关键决策

| 决策 | 选项 | 推荐 |
|:----|:----|:----|
| 数据库 ORM | Prisma / Drizzle / better-sqlite3 | **Prisma** — 声明式 Schema + 迁移 + 类型安全 |
| API 框架 | Express / Fastify / Hono | **Express** — 简历上最通用 |
| TS 运行时 | Node.js / Bun / Deno | **Node.js 20+** — 最稳定 |
| Rust CLI 调用 | `child_process` / `execa` / napi-rs | **`execa`** — Promise 包装更优雅 |
| 类型共享 | OpenAPI / tRPC / 手写 | **手写接口类型** — 前后端各一份，简单直接 |

> **桌面端：** 最终交付物为 Tauri 桌面应用。Tauri 壳负责 spawn Node.js sidecar + Vue WebView 渲染。Web 端开发时可直接 `npm run dev` 调试，打包时由 `tauri-action` 构建 `.dmg` / `.exe` / `.AppImage`。
