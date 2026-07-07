# OmniOwn

[English](README.md) | [中文](README.zh-CN.md)

**AI-Native 本地文档搜索引擎**——"AI 规划搜索，引擎执行搜索"。OmniOwn 不存储知识、不直接回答问题，而是作为搜索引擎的 AI 大脑，负责查询理解、检索规划、失败重试与结果解释。**完全本地运行，数据不出硬盘。**

---

## 架构总览

三层架构（Rust 核心 → Node.js API → Vue 前端），每层职责单一：

```
┌──────────────────────────────────────────────────────────────┐
│  Tauri v2 桌面壳 (src-tauri/)                                │
│  • 系统托盘 + 悬浮面板  • Sidecar 进程管理                    │
│  • 启动时自动拉起 Node.js API 服务                            │
└─────────────────────┬────────────────────────────────────────┘
                      │ WebView
┌─────────────────────▼────────────────────────────────────────┐
│  Vue 3 + TypeScript (ui/)                                    │
│  • SearchView / DocumentsView / ConfigView / StatusView      │
│  • Pinia 状态管理 · Element Plus · Hash 路由                  │
└─────────────────────┬────────────────────────────────────────┘
                      │ HTTP (localhost:3001)
┌─────────────────────▼────────────────────────────────────────┐
│  Node.js + Express + TypeScript API (server/)                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  AI 搜索管线                      FTS5 检索引擎         │ │
│  │  ┌──────────┐ ┌──────────────┐    • SQLite FTS5        │ │
│  │  │ 查询     │ │ 策略         │    • BM25 排序          │ │
│  │  │ 分析     │→│ 选择         │    • 摘要高亮           │ │
│  │  │ (LLM)    │ │ (LLM + zod)  │    • 8 个搜索维度       │ │
│  │  └──────────┘ └──────┬───────┘                         │ │
│  │                      │ 并行执行                          │ │
│  │                      ▼                                   │ │
│  │  ┌─────────────────────────────────────────────────────┐ │ │
│  │  │  Prisma ORM → SQLite (FTS5) · 配置 · 监听管理       │ │ │
│  │  └─────────────────────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────┬────────────────────────────────────────┘
                      │ child_process (spawn / stdio)
┌─────────────────────▼────────────────────────────────────────┐
│  Rust 核心 + CLI (src/)  —  172 单元测试，零 Clippy 警告     │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  extractor   processor    watch (notify)    MCP Server  │ │
│  │  • 10+       • Pipeline   • 递归监听       • JSON-RPC   │ │
│  │    格式       引擎                          2.0         │ │
│  │  • PDF/XLSX  • 分类      • 1s 静默检测     • 工具调用   │ │
│  │  • Office    • 元数据    • 稳定性检测       协议         │ │
│  │  • 代码      • 持久化                                      │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

---

## 核心工程亮点

### AI 搜索管线（LLM + FTS5 混合）

两阶段管线，将 "AI 搜索大脑" 变为确定性执行引擎：

```
用户输入 "我上周的代码文件"
  │
  ▼ Stage 1 — 查询分析 (LLM)
  │   改写 + 关键词提取 + 意图分类
  │   → {rewrittenQuery, keywords, intent, suggestedCategory, timeRangeDays}
  │   ↓ LLM 失败时降级：直接使用原始查询
  ▼ Stage 2 — 策略选择 (LLM + zod JSON Schema)
  │   LLM 选择最优策略组合，zod schema 校验输出格式
  │   → [{strategy: "recent", params: {days: 7}}, {strategy: "fulltext", params: {query: "代码"}}, ...]
  │   ↓ LLM 失败时降级：默认全文检索
  ▼ 并行执行 (Promise.allSettled)
  │   8 个策略跨维度并发执行
  │   → 全文 / 分类 / 文件类型 / 摘要 / 时间 / 隐私 / 文件名 / 标签
  ▼ 分层合并去重
  │   • FTS 命中 (rank ≠ -1)：全部保留
  │   • 非 FTS 命中 (rank = -1)：最多补充 5 条
  │   • 纯非 FTS（浏览模式）：不限量
  ▼ Top 20 → 返回
```

**关键设计决策：**
- **规划-执行分离**：LLM 只输出 `SearchPlan`（策略列表 + 置信度），不接触搜索执行。新增策略只需实现一个接口。
- **Prompt 变体 (v1/v2)**：模块化 Prompt 系统支持 A/B 测试。v2 增加 Few-shot 示例 + 文档统计上下文注入。
- **60 秒 TTL 缓存**：`getDocumentStats()` 结果缓存避免每次 AI 搜索都查库；文件变更时立即失效。
- **优雅降级**：AI 不可用时自动降级为纯 FTS5 搜索，搜索引擎独立工作。

### 实时索引管道 (Rust)

```
notify (文件系统事件)
  │
  ▼ 1 秒静默 + 文件大小不变
  │   稳定性检测防止正在写入的文件被提前索引
  ▼ 30 秒去重指纹
  │   防止重复事件重复处理
  ▼ extract() → classify() → persist()
  │   PipelineStep trait：新 Parser 插入 Step 链即可，不修改核心流程
  ▼ 解析失败自动隔离 → quarantine/
  │   失败文件隔离到 quarantine 目录，SSE 推送到前端
```

### MCP 协议集成

内置 MCP Server（JSON-RPC 2.0），Claude Desktop、Cursor 等 AI 客户端可直接接入本地知识库，无需额外配置。

---

## 技术栈

| 层 | 技术 | 说明 |
|:---|:-----|:-----|
| 文本提取 | Rust · lopdf · calamine · quick-xml | 10+ 格式，单二进制 |
| 文件监听 | Rust · notify (inotify/FSEvents/ReadDirectoryChanges) | 跨平台，1s 防抖 |
| 全文检索 | SQLite FTS5 · Prisma ORM v5 | BM25 排序、摘要高亮、零外部依赖 |
| AI 编排 | Node.js · Express · TypeScript | 两阶段 LLM 管线、zod 校验、Prompt 变体 |
| 前端 | Vue 3 · Pinia · Vite · Element Plus | 悬浮面板、Hash 路由 |
| 桌面壳 | Tauri v2 (tray + shell + dialog + positioner) | ~5 MB 二进制、Sidecar 进程管理 |
| 协议 | MCP (Model Context Protocol) | JSON-RPC 2.0、工具调用模式 |
| CI/CD | GitHub Actions | Release 自动构建 Windows 安装包 |

---

## 质量指标

| 指标 | 数值 |
|:----|:-----|
| Rust 单元测试 | 172 |
| TypeScript 严格模式 | 已启用 |
| Clippy | 零警告目标 |
| Rustfmt | 强制检查 |
| Windows Release 构建 | 支持 |

---

## 快速开始

```bash
git clone https://github.com/zj-rrissh/omniown.git
cd omniown

npm --prefix server install
npm --prefix ui install
npm --prefix server run build
npm --prefix ui run build
cargo build
```

或从 [Releases](https://github.com/zj-rrissh/omniown/releases) 下载 Windows 安装包。

---

## 文档

[架构](docs/architecture.md) · [CLI](docs/cli.md) · [配置](docs/config.md) · [数据库](docs/database.md) · [开发](docs/development.md) · [提交记录](docs/git-history.md) · [变更日志](CHANGELOG.md)
