# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.3] - 2026-06-15

### Added

- Add an explicit AI search mode toggle and visible AI trace output for strategy selection and execution results.

### Fixed

- Send AI searches through `GET /api/search?q=...&ai=true` so the AI multi-strategy path is actually used.
- Suppress repeated watcher output for unchanged files and already-recorded extraction failures after bulk imports.

## [0.1.2] - 2026-06-09

### Changed

- Publish Windows releases with both MSI and NSIS setup.exe installers.

### Fixed

- Keep the Windows watcher/config fixes from 0.1.1 in the installer build.

## [0.1.1] - 2026-06-09

### Fixed

- Fix Windows packaged startup config generation so default library paths are TOML-safe.
- Restart the file watcher after saving path settings so custom library directories take effect.
- Ensure packaged `omniown watch` receives the configured library path instead of falling back to `.\library`.

## [0.1.0] — 2026-06-08

### Added

- **全栈架构** — Rust CLI（文本提取 + 文件管线 + MCP）→ Node.js API（Express + Prisma）→ Vue 3 前端 + Tauri v2 桌面壳
- **FTS5 全文搜索** — SQLite FTS5 虚拟表 + 触发器自动同步，毫秒级查询
- **AI 多策略搜索** — LLM 分析意图，从 8 种策略中选择最优组合，并行执行合并去重
- **MCP Server** — 4 工具（search_documents / get_document / list_documents / get_status），AI 客户端直接接入本地知识库
- **多格式文本提取** — 纯文本、Markdown、HTML、代码、JSON/YAML/TOML/CSV、PDF、DOCX、XLSX
- **文件导入管线** — SHA256 去重、自动分类（公开/私密）、同名冲突交互处理
- **Node.js REST API** — Express 5 + Prisma 5，4 路由（status / documents / search / config），TypeScript strict
- **Vue 3 前端** — Pinia 状态管理，4 视图（搜索 / 文档 / 配置 / 状态），Vite 6 构建
- **Tauri v2 桌面应用** — 系统托盘 + 悬浮面板，spawn Node.js 子进程作为 API 服务
- **TOML 配置管理** — 自定义路径（root / library）、LLM API 配置
- **GitHub Actions CI** — fmt → test → clippy 三步检查
- **Release CI** — Windows 构建，sidecar + server + ui 打包；Linux/macOS 配置保留为注释
- **系统目录选择器** — 设置页可选择 library 路径
- **library 实时同步** — `omniown watch` 递归监听 library 目录，新增文件自动索引，删除文件自动清理数据库记录
- **启动兜底** — 首次启动自动创建默认配置文件；缺少 Node.js 时弹窗提示
- **开发治理文档** — 新增 AI 开发治理框架
- **提交记录总结** — 新增 [Git 提交记录总结](docs/git-history.md)，按阶段梳理 112 条提交的项目演进

### Changed

- 架构从 Rust 单体重构为三层全栈
- 数据库模块精简：删除 migration / classifier / storage，逻辑内联
- 前端从直接 API 调用重构为 Pinia stores + service 层
- 响应字段 snake_case → camelCase（匹配 Prisma 原生输出）
- 取消 inbox 概念，改为直接管理 library 目录
- README 和 `.gitignore` 按当前架构重新整理，补充 `.env.example` 和 SQLite WAL/SHM 忽略规则

### Removed

- Embedding 向量化搜索（由 AI 多策略搜索替代）
- 文件监控哨兵模式（由 Node.js API + CLI 手动导入替代）
- `src/migration.rs` / `src/classifier.rs` / `src/storage.rs`（逻辑内联）

### Fixed

- 前后端 API 响应格式不匹配（外层包装 + 字段名）
- CI `package-lock.json` gitignore 导致 `npm ci` 失败
- Release CI 侧车路径不匹配 — tauri-action 缺少 `--target` 导致架构名错位
- Release CI macOS x86_64 构建切换到 `macos-15-intel` 跑者
- GitHub Actions 版本升级（Node.js 20 弃用）：checkout@v6, upload-artifact@v7, download-artifact@v8, setup-node@v6
- Release CI write permissions / artifactPath / 打包类型 等配置修复
- 桌面打包后后端无法启动：修复 packaged API endpoint、Node.js 入口路径、bundled Node runtime 和 `omniown` sidecar 打包
- 打包后的 watch 配置路径，确保安装环境中能正确读取 library 配置
- 双托盘图标、窗口置顶/启动显示/透明拖动行为
- Node.js 服务首次启动数据库初始化问题
- fmt/clippy lint 与 release CI 中 WiX Toolset 下载/预装检测问题
