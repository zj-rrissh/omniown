# 开发文档

## 环境要求

| 部分 | 语言 | 版本要求 |
|:---|:---|:---|
| 后端 API | Node.js + TypeScript | Node.js 20+ |
| 前端 | Vue 3 + TypeScript | Node.js 20+ |
| Rust Core + CLI | Rust | 1.85+ (stable) |
| Tauri 桌面壳 | Rust (Tauri v2) | 1.85+ |

## 快速开始

```bash
# 安装依赖
npm --prefix server install
npm --prefix ui install

# 构建 Rust Core + CLI
cargo build
```

## 启动开发服务

### 后端 API

```bash
cd server
npm run dev
# → http://127.0.0.1:3001
```

### 前端

```bash
cd ui
npm run dev
# → http://localhost:5173（自动代理 /api → 127.0.0.1:3001）
```

### Tauri 桌面端

```bash
cargo tauri dev --config src-tauri/tauri.conf.json
```

## 构建

### 生产构建

```bash
# 1. 构建 Rust Core + CLI
cargo build --release

# 2. 构建后端
npm --prefix server run build

# 3. 构建前端
npm --prefix ui run build

# 4. Tauri 打包（含 server/ + ui/ + `omniown` CLI sidecar）
cargo tauri build --config src-tauri/tauri.conf.json
```

### 构建产物

| 构建 | 产物 |
|:---|:---|
| Rust Core + CLI | `target/release/omniown` |
| 后端 | `server/dist/` |
| 前端 | `ui/dist/` |
| Tauri 桌面端 | `src-tauri/target/release/bundle/` |

## 数据库操作

```bash
# 同步 Schema 到数据库（幂等）
cd server && npx prisma db push

# 可视化浏览
cd server && npx prisma studio

# 查看 SQLite 数据
sqlite3 server/prisma/dev.db "SELECT COUNT(*) FROM documents;"
```

## 代码检查

### TypeScript

```bash
npm --prefix server run build    # tsc 类型检查
npm --prefix ui run build         # vue-tsc + vite build
```

### Rust

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## 项目结构

```
omniown/
├── server/               # Node.js/TS API (Express + Prisma)
│   ├── src/
│   │   ├── index.ts             # 入口：Express 启动 + DB init + 路由挂载
│   │   ├── api/                 # 路由层
│   │   │   ├── status.ts        # GET /api/status
│   │   │   ├── documents.ts     # GET /api/documents[/:id]
│   │   │   ├── search.ts        # GET /api/search[?q=&ai=true]
│   │   │   └── config.ts        # GET/PUT /api/config
│   │   ├── services/            # 业务逻辑层
│   │   │   ├── search.service.ts # FTS5 搜索（8 策略）
│   │   │   ├── ai.service.ts     # LLM 策略选择
│   │   │   └── import.service.ts # `omniown` CLI 编排
│   │   ├── db/
│   │   │   ├── client.ts         # Prisma 客户端
│   │   │   └── setup-fts.ts      # FTS5 虚拟表初始化
│   │   ├── config/
│   │   │   └── index.ts          # TOML 配置读写
│   │   └── middleware/
│   │       └── error.ts          # 错误处理
│   └── prisma/
│       ├── schema.prisma         # 数据库 Schema
│       └── dev.db                # SQLite 数据库（gitignore）
├── ui/                   # Vue 3 + TypeScript 前端
│   └── src/
│       ├── App.vue               # 壳布局（托盘图标 + 导航）
│       ├── router.ts             # Hash 路由
│       ├── views/                # 4 个页面
│       │   ├── SearchView.vue    # 搜索首页
│       │   ├── DocumentsView.vue # 文档列表
│       │   ├── ConfigView.vue    # 设置页面
│       │   └── StatusView.vue    # 系统状态
│       ├── services/             # API 客户端
│       │   ├── api-client.ts     # fetch 封装
│       │   └── config.service.ts # 配置 API
│       └── stores/               # Pinia 状态
├── src/                  # Rust Core + CLI
│   ├── lib.rs                    # omniown_core library 入口
│   ├── runtime.rs                # 推荐外部复用门面
│   ├── main.rs                   # CLI 入口
│   ├── extractor.rs              # 文本提取
│   ├── processor.rs              # 文件管线
│   ├── watch.rs                  # 文件夹监听
│   └── mcp.rs                    # MCP Server
├── src-tauri/            # Tauri v2 桌面壳
│   ├── src/main.rs               # 壳逻辑 + sidecar 管理 + Tauri 命令
│   ├── capabilities/             # 权限声明
│   ├── tauri.conf.json           # Tauri 配置
│   └── Cargo.toml
├── docs/                 # 项目文档
└── .github/workflows/    # CI/CD
    ├── ci.yml                    # PR 检查
    └── release.yml               # Tauri Release 打包（当前聚焦 Windows）
```

## 开发原则

1. **路由层不写业务逻辑** — `api/*.ts` 只做 HTTP 编排，调用 `services/`
2. **服务层不碰 HTTP** — `services/*.ts` 调用数据库和外部 API，不接触 req/res
3. **LLM 不写 SQL** — AI 选择策略名，由服务层执行具体 SQL
4. **Rust Core 做重型处理** — 文本提取、文件管线、监听和 MCP 位于 `omniown_core`，CLI 保持为 Node.js/Tauri 的兼容入口

## CI

- `ci.yml` — PR 代码检查 (fmt + test + clippy)
- `release.yml` — Tauri 桌面端打包（当前聚焦 Windows；macOS / Linux 配置保留为注释）
