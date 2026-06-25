# Git 提交记录总结

> 基于当前 `main` 分支截至 `861151b` 的 Git 历史整理。

## 概览

| 项目 | 内容 |
|:---|:---|
| 提交数量 | 150+ |
| 时间跨度 | 2026-05-21 至 2026-06-25 |
| 当前分支 | `main` |
| 当前标签 | `v0.1.4` |
| 最新提交 | 两阶段 AI 搜索管线 + zod 验证 + 分层合并 |

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

## 6. 配置路径收束与文档国际化（2026-06-10）

- 统一配置文件名 `config.example.toml` → `omniown.example.toml`，数据库命名规范化。
- `server/src/config/index.ts` 新增 TOML 配置读写能力，与 Rust CLI 共享同一配置源。
- 增加英文版 README、CONTRIBUTING，补齐国际化入口。
- 发布 v0.1.1（Windows 打包修复）、v0.1.2（配置路径规范化）。

## 7. AI 搜索增强与 UI 组件化（2026-06-15 至 2026-06-22）

- **AI 搜索可观测性：** 增加 SearchTrace 追踪每次 AI 搜索的推理链路（query→rewrite→decompose→synthesis），前端展示思考过程。watch 日志降噪，减少终端干扰。
- **Rust Kernel Plan：** 规划 extractor 能力下沉到 Rust 层的长期路线，新增 `src/runtime.rs` 运行时模块，为 CLI 与 Tauri sidecar 统一入口做准备。
- **UI 组件化：** 引入 Element Plus 组件库，搜索页和文档页全量替换为 El-* 组件（El-drawer、El-pagination、El-tag、El-scrollbar 等），底部导航栏改用 Element Plus 图标，设置页和状态页同步优化。
- **AI Prompt 模块化：** 将 AI 搜索 System Prompt 从 `ai.service.ts` 硬编码分离到独立模块 `search-strategy.prompt.ts`，支持 v1/v2 变体 + 6 个 Few-shot 示例 + 文档库上下文注入 + fallback 兜底。
- **全量文档同步：** 更新 architecture.md、cli.md、config.md、database.md、development.md、migration-plan.md、git-history.md 到当前实际架构。
- 发布 v0.1.3（AI 搜索追踪 + watch 日志降噪）。

## 8. 实时同步与 UI 体验打磨（2026-06-22 至 2026-06-24）

- **SSE 自动刷新：** 新增 `server/src/services/events.service.ts` SSE 客户端管理，`GET /api/events` 端点推送文件变更通知。watch 解析 stdout 检测新增/删除事件时广播。前端 SSE 客户端自动重连，文档/搜索页收到通知自动刷新列表。
- **无限滚动加载：** 文档列表从 `el-pagination` 分页改为 IntersectionObserver 无限滚动。API 支持 `?limit=&skip=` 分页参数，store 新增 `loadInitial/loadMore/reload/hasMore` 状态管理。
- **修复文档列表截断：** API 原硬编码 `take:20` 按 `updatedAt DESC` 排序，导致更新时间较早的文档被截在第 21 位永远不可见。改为读取前端 `?limit=` 参数（默认 200），前端 store 自行分页。
- **Config 安全合并：** `saveConfig` 从全量替换改为 `deepMerge`，避免 `prompt_variant` 等非前端管理字段在保存配置时被静默清除。
- **Watch 删除路径对齐：** `stored_path_for_db` 写入绝对路径而 `handle_remove` 查询相对路径，导致删除 library 文件后 DB 记录残留。改为统一使用 `processor::stored_path_for_db()` 计算路径。
- **提取失败降级：** `handle_extraction_failure` 中 `std::fs::rename` 移动文件后原路径 `metadata` 读取失败导致 `file_size` 始终为 `null`。改为在移动文件前捕获 `file_size`，移动后从 `stored_path` 读取扩展名。

## 9. 两阶段 AI 搜索管线（2026-06-25）

- **Stage 1 查询分析：** 新增 `prompts/query-analysis.prompt.ts` 和 `analyzeQuery()` 函数。LLM 改写用户查询并提取结构化关键词/意图/分类/文件类型/时间范围/隐私偏好。失败时自动降级使用原始查询，不阻塞搜索。
- **Stage 2 策略选择 + JSON Schema 验证：** 新增 `utils/validate-strategies.ts`，引入 zod 对 LLM 策略输出进行运行时校验（策略名 enum 检查 + 最少 1 策略 + params 结构验证），替代裸 `as StrategyCall[]` 类型断言。验证失败抛出含详细 issue 的错误。
- **Prompt 模块化 + 国际化：** 将 AI System Prompt 从 `ai.service.ts` 硬编码分离到独立 `prompts/` 模块。新增 `index.ts` barrel 导出。全部 Prompt 改为英文指令 + 中英混合 Few-shot 示例，提升 JSON 输出合规率。
- **v2 文档库上下文注入：** `getDocumentStats()` 提供文档总数和已有分类列表，注入 v2 System Prompt 的 `[Document Library Info]` 块。结果 60 秒 TTL 缓存，避免每次 AI 搜索都查询数据库。
- **缓存失效管理：** `clearDocStatsCache()` 在文档导入成功（`import.service.ts`）和文件监听事件（`watch-manager.ts`）时调用，确保缓存与数据库一致。
- **分层结果合并：** FTS 命中（rank ≠ -1）全保留，非 FTS 命中（分类/文件类型等，rank = -1）最多 5 条补充。纯非 FTS 搜索（浏览类）不限量，保留原有浏览体验。

## 主题归类

| 主题 | 代表提交 |
|:---|:---|
| 文档管线与索引 | `2da6e93`, `f1bddc3`, `f24ba94`, `1059a9e` |
| 搜索能力 | `1ad848e`, `2e2a910`, `e7d2241`, `c0eacdb`, `a4c2fcf`, `c22f544` |
| MCP 与 CLI | `d78dfce`, `550ae8c`, `998348d` |
| 桌面端 | `7214813`, `4d58122`, `f50ddfd`, `341ac44` |
| 全栈迁移 | `907ff38`, `0e3048f`, `998348d`, `341ac44` |
| Library 实时同步 | `ebb99b1`, `20c1820`, `810230a`, `7491438` |
| CI/CD 与发布 | `edfd1bb`, `6d29485`, `ab8755e`, `4222f9b`, `7d36267`, `c4f6fb9`, `dd6ea1c` |
| Windows 打包修复 | `5e76050`, `ffca7aa`, `520712b`, `67a3d3c` |
| UI 组件化 | `600c629`, `cf299e6`, `78b108b`, `1656e80` |
| 实时同步（SSE） | `62eaba2` |
| AI Prompt 模块化 | `c22f544` |
| 两阶段 AI 搜索管线 | zod 验证 + 查询分析 + 分层合并 + 缓存管理 |
| AI 搜索可观测性 | `a4c2fcf` |
| Rust 运行时规划 | `68d03fa` |
| 配置安全合并 | `06bdf59` |

## 当前结论

OmniOwn 的提交历史显示项目已从早期 Rust 本地文件处理工具，演进为面向桌面交付的全栈本地文档管理应用。当前重点已经从功能搭建转向 UI 体验优化和搜索质量提升，同时持续修复各种边界条件 bug。

**近期进展（2026-06-24）：**
- 引入 Element Plus 组件库，全量替换搜索/文档/设置/状态页的原生组件
- 底部导航栏改用 Element Plus 图标，详情面板改用 El-drawer 抽屉组件
- AI 搜索增加 SearchTrace 推理链路追踪，前端展示思考过程
- 文档列表从分页组件改为 IntersectionObserver 无限滚动加载
- 实现 SSE 自动刷新，library 文件变更时前端文档/搜索页自动更新
- 修复文档列表 API 硬编码 `take:20` 导致部分文档永久不可见
- saveConfig 改为 deepMerge，防止非前端管理字段被覆盖丢失
- AI 搜索 System Prompt 分离到独立模块，支持 v1/v2 变体
- Rust Kernel Plan 启动，新增 runtime 运行时模块
- 发布 v0.1.1、v0.1.2、v0.1.3，增加英文文档
