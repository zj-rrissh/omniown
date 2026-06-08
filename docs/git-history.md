# Git 提交记录总结

> 基于当前 `main` 分支截至 `67a3d3c` 的 Git 历史整理。

## 概览

| 项目 | 内容 |
|:---|:---|
| 提交数量 | 112 |
| 时间跨度 | 2026-05-21 至 2026-06-08 |
| 当前分支 | `main` |
| 当前标签 | `v0.1.0` |
| 最新提交 | `67a3d3c fix: repair packaged watch config paths` |

## 阶段演进

### 1. Rust 本地文档管线奠基（2026-05-21 至 2026-05-23）

- 初始化项目，并完成事件分流、类型过滤、防抖与异步事件处理。
- 建立 SQLite 持久化、目录布局管理、文件分层存放和数据库索引。
- 引入 FTS5 全文检索、Embedding Pipeline 骨架、Lazy Idle Worker 与本地 embedding provider。
- 扩展文本提取能力，覆盖更丰富的纯文本、代码和文档格式。
- 建立第一批 README、架构、CLI、配置、数据库、开发文档，并加入 GitHub Actions 基础 CI。

### 2. 搜索、MCP 与桌面端原型（2026-05-23 至 2026-05-25）

- 完成语义搜索、AI 智能搜索、文档同步和数据库文件一致性修复。
- 剥离 embedding 向量化代码，转向 AI 多策略搜索方向。
- 增加 MCP Server，以及 PDF/Office 文档解析能力。
- 分阶段实现 Tauri 托盘悬浮面板、sidecar 自动启停、LLM 配置界面、MCP 管理、四标签导航和文档分页过滤。
- 补齐单元测试、验收清单、CHANGELOG 和发布 CI，同时修复安全、HTTP 服务、Tauri 编译、图标和 Linux CI 依赖等问题。

### 3. 全栈迁移与架构收敛（2026-05-27 至 2026-05-31）

- 明确最终交付形态为本地桌面工具：Tauri + Node.js sidecar + Vue 前端，无需认证。
- 清理冗余目录和过期个人文档，将 Rust 单体拆分为 Rust CLI、Node.js API、Vue 3 前端与 Tauri 桌面壳。
- 增加 `import.service.ts`，精简 Rust 核心，删除已被 Node.js API 替代的模块。
- 完成 status/search 路由、FTS5 初始化、Express 路由归一和 Pinia/service 前端分层。
- 补齐开源项目要素：LICENSE、CHANGELOG、CONTRIBUTING、Code of Conduct 和文档修正。
- 修复 release CI 中的 artifactPath、tauri-action inputs、打包类型、写权限、sidecar 路径、actions 版本升级、WebView2 和 capabilities 兼容问题。

### 4. Library 直管与实时同步（2026-06-01 至 2026-06-05）

- 修复双托盘图标、窗口置顶/显示/透明拖动行为，以及 Node.js 服务首次启动自动初始化数据库。
- 设置页增加系统目录选择器，让用户可自由选择存储路径。
- 文档全量更新到当前架构，并标注三个最终目标。
- 引入 `omniown watch` 文件夹监听与自动导入，取消 inbox 概念，改为直接管理 library 目录。
- 删除 library 文件时自动清理数据库记录，新增问题追踪文档并记录典型经验。
- 整理 `.gitignore`，覆盖 SQLite WAL/SHM、OS/IDE 通用忽略项，并修复 fmt/clippy lint。

### 5. Windows 打包和运行时修复（2026-06-07 至 2026-06-08）

- 修复桌面打包后后端无法启动的多个问题。
- 首次启动自动创建默认配置文件，并在缺少 Node.js 时弹窗提示。
- Release 构建暂时聚焦 Windows，Linux/macOS 配置保留为注释。
- 打包 bundled `omniown` sidecar，并在桌面构建中使用打包后的 API endpoint。
- 规范 Windows 下 Node.js 入口路径，打包 Node runtime，并修复 Node runtime 下载版本解析。
- CI 对 WiX Toolset 下载增加重试，并优先使用预装 WiX。
- 修复打包后的 watch 配置路径，确保 library 监听与配置读取在安装环境中一致。

## 主题归类

| 主题 | 代表提交 |
|:---|:---|
| 文档管线与索引 | `2da6e93`, `f1bddc3`, `f24ba94`, `1059a9e` |
| 搜索能力 | `1ad848e`, `2e2a910`, `e7d2241`, `c0eacdb` |
| MCP 与 CLI | `d78dfce`, `550ae8c`, `998348d` |
| 桌面端 | `7214813`, `4d58122`, `f50ddfd`, `341ac44` |
| 全栈迁移 | `907ff38`, `0e3048f`, `998348d`, `341ac44` |
| Library 实时同步 | `ebb99b1`, `20c1820`, `810230a` |
| CI/CD 与发布 | `edfd1bb`, `6d29485`, `ab8755e`, `4222f9b`, `7d36267` |
| Windows 打包修复 | `5e76050`, `ffca7aa`, `520712b`, `67a3d3c` |

## 当前结论

OmniOwn 的提交历史显示项目已从早期 Rust 本地文件处理工具，演进为面向桌面交付的全栈本地文档管理应用。当前重点已经从功能搭建转向发布稳定性，尤其是 Windows 打包、sidecar 路径、Node runtime、默认配置和 watch 配置路径等安装环境问题。
