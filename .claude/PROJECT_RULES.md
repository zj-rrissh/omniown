# OmniOwn — 项目规则

## 项目目标

- **解决的问题**：AI 驱动的本地文档管理与知识库检索——文件放入即自动索引，全文搜索 + AI 多策略检索
- **目标用户**：希望用 AI 管理本地文档的个人用户（研究者、写作者、开发者）
- **核心价值**：完全本地化（隐私优先）、零配置自动索引（放入即用）、FTS5 全文搜索 + AI 多策略检索、MCP 协议集成（可接入 Claude Desktop 等 AI 工具）

## 技术栈

| 层 | 技术 | 版本 | 用途 |
|------|------|------|------|
| 桌面壳 | Tauri v2 | 2.x | 窗口管理、托盘、进程生命周期、系统对话框 |
| 前端 | Vue 3 + Vite 6 | 3.5 / 6.x | WebView UI、Pinia 状态管理、Vue Router 路由 |
| API 服务 | Node.js + Express 5 | 20+ / 5.0 | REST API、文件监听进程管理、配置管理 |
| ORM | Prisma | 5.22 | SQLite 数据库操作、Schema 迁移 |
| 搜索引擎 | SQLite FTS5 | 内置 | 全文搜索、BM25 排序 |
| CLI | Rust | 1.85+ | 文件处理、文本提取、文件夹监听、MCP Server |
| 关键依赖 | rusqlite (bundled) | 0.31 | Rust 端数据库访问（与 Prisma 共用 WAL 模式） |
| 关键依赖 | notify | 7 | 跨平台文件系统事件监听（inotify/FSEvents/ReadDirectoryChanges） |
| 关键依赖 | @tauri-apps/api | 2.x | 前端调用 Tauri 原生能力 |
| 关键依赖 | tauri-plugin-shell | 2.x | Tauri 侧启动 Node.js 进程 |
| 关键依赖 | tauri-plugin-dialog | 2.x | 系统原生目录选择对话框 |
| 关键依赖 | tauri-plugin-positioner | 2.x | 悬浮面板位置管理 |
| 关键依赖 | @iarna/toml | - | Node.js 端 TOML 配置读写 |
| 关键依赖 | serde + toml | 1 / 0.8 | Rust 端 TOML 配置读写 |

## 架构约束

### 模块依赖规则

- **依赖方向**：单向，无循环依赖。前端 → API → 数据库；Tauri → Node.js → Rust CLI
- **模块通信**：通过接口（API endpoint / IPC command / CLI args），不直接依赖实现
- **公共模块**：`server/src/services/` 不依赖 `server/src/api/`；`server/src/db/` 不依赖业务模块

### 数据流

```
用户放入文件到 library/
  → Rust CLI (omniown watch) 检测到文件变更
  → 稳定性检测（1s 无变化）
  → 提取文本（extractor.rs）
  → 分类（category/domain/docType）
  → SHA256 去重
  → 写入 SQLite（rusqlite）
  → FTS5 触发器自动同步索引
  → Node.js API 从同一 DB 读取
  → Vue 前端通过 http://127.0.0.1:3001 获取数据
```

### 进程模型

```
Tauri Shell (主进程)
├── Node.js API (子进程, 端口 3001, 自动重启 5 次)
│   └── omniown watch (孙进程, 文件夹监听)
└── omniown mcp (可选, 用户手动启停)
```

### 数据库约束

- **WAL 模式必须**：Prisma 和 rusqlite 并发访问同一 SQLite 文件，必须都是 WAL journal 模式
- **数据库路径统一**：`DATABASE_URL` 统一为绝对路径，通过环境变量或 CLI args 传递
- **Schema 迁移**：Node.js 启动时自动执行 `prisma db push --skip-generate`（幂等安全）
- **FTS5 触发器的创建**：在 Node.js 端通过 `setup-fts.ts` 执行（Prisma 不支持 FTS5）

## 技术债务管理

- TODO/FIXME/HACK 注释建议关联 Issue 编号
- 每个迭代预留 10-20% 处理技术债务
- 新依赖需评估：是否必要？是否活跃维护？License 兼容？
- 当前已知债务：`import.service.ts` 无调用者（死代码）、`DocumentEmbedding` 表已废弃、`config.example.toml` 含旧 inbox 字段

## 性能原则

- 先保证正确性，再优化性能
- 优化必须有 profiling 数据支撑
- 数据库查询：避免 N+1，注意 FTS5 查询效率
- 文件监听：稳定性检测 1s + 去重 800ms 防止重复索引
- 前端：路由懒加载、内容截断显示（100KB limit）

## 安全原则

- 用户输入：never trust，始终校验和转义
- 敏感数据：API key 不在 IPC 中明文返回（前端仅显示前 4 位 + `***`）
- 配置文件：`omniown.toml` 和 `server/.env` 加入 .gitignore
- 依赖安全：定期运行 `cargo audit`、`npm audit`
- API 安全：CORS 白名单（CSP: `connect-src 'self' http://127.0.0.1:3001`）、输入大小限制
- 桌面端：panic 弹窗（Windows MessageBoxW）防止静默崩溃
