# OmniOwn — AI 协作指南

## 项目概要

OmniOwn 是一个基于 Tauri v2 的 AI 驱动本地文档管理桌面应用。三层架构：Tauri Shell（Rust）→ Node.js API 服务（Express 5 + Prisma + SQLite FTS5）→ Vue 3 前端（WebView）。Rust CLI 提供文件处理、文本提取、文件夹监听、MCP Server 等能力。

## 工作流

### 日常开发流程

需求/Issue → 分析设计（先出方案）→ 小步编码 → 自测 → Review → 提交

### 新功能开发规范

1. **分析阶段**：阅读相关代码，理解现有架构，输出设计要点（不写代码）
2. **设计阶段**：确定接口/数据模型/文件变更清单，确认后再编码
3. **编码阶段**：每完成一个独立单元（≤200 行），停下来验证
4. **自测阶段**：逐项对照设计要点检查，确认无遗漏
5. **测试阶段**：运行全部测试，确认无回归
6. **提交阶段**：小原子提交——一次提交做一件事

### Bug 修复规范

1. **复现**：写出复现步骤或复现测试用例
2. **定位**：通过日志/断点/bisect 找到根因——不修症状
3. **修复**：最小改动——不顺便重构无关代码
4. **回归**：添加回归测试防止复发
5. **记录**：非显而易见的坑写入 lessons-learned

### Code Review 规范

- AI 生成代码必须经过人工 review（或使用 /code-review skill）
- Review 检查点：逻辑正确性、安全漏洞、性能陷阱、边界条件处理
- 超过 200 行变更应分批 review
- 所有 review 意见必须在合并前解决

### 测试规范

- 新功能必须有测试（优先 TDD）
- Bug 修复必须先写复现测试
- 不得为了通过测试而降低断言标准
- CI 测试必须 100% 通过

### Rust 编码规范

- 使用 edition 2024（CLI）/ edition 2021（Tauri Shell）
- 运行 `cargo fmt -- --check` + `cargo clippy -- -D warnings` 提交前
- 测试写在 `#[cfg(test)] mod tests` 内联块中
- 所有 pub 函数必须有文档注释

### TypeScript 编码规范

- 严格模式（strict: true）
- ESM 模块（NodeNext for server, Bundler for ui）
- Express 路由与 Prisma 查询分离（路由不直接写 SQL）
- 新 API 端点需同步更新 api-docs

## 项目结构速查

```
omniown/
├── src/                     # Rust CLI（8 文件）
│   ├── main.rs              # 入口 + 5 子命令
│   ├── config.rs            # TOML 配置加载
│   ├── db.rs                # rusqlite 数据库操作
│   ├── extractor.rs         # 10+ 格式文本提取
│   ├── fs_layout.rs         # AppPaths 路径解析
│   ├── mcp.rs               # MCP Server (JSON-RPC 2.0)
│   ├── processor.rs         # 文件导入管道
│   └── watch.rs             # 文件夹监听（notify）
├── src-tauri/               # Tauri v2 桌面壳
│   └── src/main.rs          # 窗口/托盘/进程管理
├── server/                  # Node.js API 服务
│   ├── src/index.ts         # Express 入口（端口 3001）
│   ├── src/api/             # 4 个路由模块
│   ├── src/services/        # AI 搜索、搜索服务
│   ├── src/db/              # Prisma 客户端 + FTS5
│   ├── src/config/          # TOML 配置读写
│   ├── src/middleware/       # 错误处理、日志
│   └── prisma/schema.prisma # 数据模型
├── ui/                      # Vue 3 前端
│   └── src/
│       ├── views/           # 4 个页面视图
│       ├── services/        # 5 个 API 服务
│       └── stores/          # 2 个 Pinia store
├── docs/                    # 项目文档（7 篇）
└── .claude/                 # AI 治理框架（本目录）
    ├── CLAUDE.md
    ├── PROJECT_RULES.md
    ├── skills/
    └── knowledge/
```

## 常用命令

```bash
# 开发
cargo build                          # 编译 Rust CLI
cd src-tauri && cargo build          # 编译 Tauri Shell
cd server && npm run dev             # 启动 API（热重载）
cd ui && npm run dev                 # 启动前端（Vite）

# 测试
cargo test                           # Rust 测试（172 个）
cd server && npx tsc --noEmit        # API 类型检查
cd ui && npx vue-tsc --noEmit        # 前端类型检查

# 代码质量
cargo fmt -- --check                 # Rust 格式检查
cargo clippy -- -D warnings          # Rust lint
cd server && npm run build           # API 编译（prisma generate + tsc）
cd ui && npm run build               # 前端编译
```
