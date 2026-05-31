# 开发文档

## 开发环境

本项目的开发环境分两部分：

| 部分 | 语言 | 版本要求 |
|:---|:----|:-----------------|
| 后端 API | Node.js + TypeScript | Node.js 20+ |
| 前端 | Vue 3 + TypeScript | Node.js 20+ |
| 核心 CLI | Rust | Rust 1.85+ (stable) |
| 桌面壳 | Rust (Tauri) | Rust 1.85+ |

## 启动开发服务

### 后端 API

```bash
cd server
npm install
npm run dev
# → http://127.0.0.1:3001
```

### 前端

```bash
cd ui
npm install
npm run dev
# → http://localhost:5173
```

### 数据库管理

```bash
# 同步 Schema 到数据库
cd server && npx prisma db push

# 可视化浏览数据
cd server && npx prisma studio
```

### Rust 核心

```bash
# 构建 CLI
cargo build

# 运行文件处理
cargo run -- process <文件路径>

# 启动 MCP Server
cargo run -- mcp
```

## 代码检查

### TypeScript

```bash
# 类型检查
npm --prefix server run build

# 格式化（与 ESLint/Prettier 集成时再加）
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
├── server/               # Node.js/TS API
│   ├── src/
│   │   ├── index.ts             # Express 入口
│   │   ├── api/                 # 路由层 (HTTP 请求/响应)
│   │   ├── services/            # 业务逻辑层 (搜索/导入/配置)
│   │   ├── db/                  # Prisma 客户端
│   │   ├── config/              # TOML 配置读取/写入
│   │   └── middleware/          # 错误处理/日志
│   └── prisma/                  # Schema + 迁移
├── ui/                   # Vue 3 + TypeScript 前端
├── src/                  # Rust 核心 CLI (缩减版)
│   ├── extractor.rs
│   ├── processor.rs
│   └── mcp.rs
├── src-tauri/            # Tauri v2 桌面壳
└── docs/                 # 文档
```

## 开发原则

### 分层原则

1. **路由层不写业务逻辑** — `api/*.ts` 只做 HTTP 编排，调用 `services/`
2. **服务层不碰 HTTP** — `services/*.ts` 调用数据库和外部 API，不接触 req/res
3. **LLM 不写 SQL** — AI 选择策略名，由服务层执行具体 SQL
4. **Rust 做重型处理** — 文本提取、文件管线用 Rust CLI，Node.js 调 `child_process`

### 数据库原则

1. **Schema 用 Prisma 声明** — 不手写 SQL 建表
2. **FTS5 用 raw query** — Prisma 不支持 FTS5，用 `$queryRaw`
3. **已有数据库兼容** — 字段名用 `@map` 映射到现有 snake_case 表结构

## CI

项目使用 GitHub Actions，配置文件：

- `ci.yml` — 代码检查 (fmt + test + clippy)
- `release.yml` — Tauri 桌面端打包

## 下一步

详见 [migration-plan.md](./migration-plan.md)
