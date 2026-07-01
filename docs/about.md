# OmniOwn

> 你的本地知识库，交给 AI 打理。

---

## 一句话

**OmniOwn** 是一个 AI 驱动的本地文档管理工具。把文件拖进知识库文件夹，它会自动索引；用日常语言描述你要找的内容，AI 帮你精确定位。**完全本地运行，数据不出你的硬盘。**

---

## 诞生背景

日常工作中，我们会在电脑上积累大量文档：Markdown 笔记、PDF 报告、代码片段、CSV 数据、个人日记…… 这些文件散落在不同目录，操作系统自带的搜索只能按文件名匹配，无法理解内容语义。

传统方案要么依赖云端（隐私风险），要么需要复杂的标签/分类体系（维护成本高）。OmniOwn 的目标很简单：**文件扔进目录就能搜，用自然语言就能找到。**

---

## 核心设计理念

### 1. 本地优先

所有数据存储在本地 SQLite 数据库中，不依赖任何云服务。向量化搜索已被弃用——我们认为 LLM 意图理解 + FTS5 全文检索的组合，在本地文档管理场景下更轻量、更可预测。

### 2. 语义理解 ≠ 向量化

OmniOwn 不 Embedding、不建向量索引。它用两阶段 AI 管线实现语义搜索：

```
用户查询 → LLM 分析意图（改写 + 关键词提取）→ 选择搜索策略 → FTS5 执行 → 合并去重
```

相比传统 RAG 方案，省去了 Embedding 模型部署、向量数据库维护、切片策略调优等复杂工作。对"个人知识库"这个量级（千到万篇文档），效果足够好，成本足够低。

### 3. 搜索策略池

不像传统搜索只用关键词匹配，OmniOwn 设计了 **8 种搜索策略**：

| 策略 | 作用 | 示例查询 |
|:---|:---|:---|
| `fulltext` | FTS5 全文检索（核心） | "关于 Kubernetes 的笔记" |
| `category` | 按分类过滤 | "找代码相关的文件" |
| `filetype` | 按文件类型过滤 | "所有 PDF 报告" |
| `recent` | 按时间范围过滤 | "上周写的日记" |
| `privacy` | 按隐私级别过滤 | "我的私密文件" |
| `filename` | 按文件名搜索 | "文件名含预算的文件" |
| `summary` | 按摘要搜索 | "关于机器学习的总结" |
| `tag` | 按标签搜索 | "标签为重要的工作文件" |

LLM 根据用户查询动态组合这些策略，并行执行，结果合并去重后返回。

### 4. 零配置运行

```bash
# 启动 API 服务
cd server && npm run dev

# 启动前端
cd ui && npm run dev

# 把文件放入 library/，自动索引
cp ~/Documents/note.md library/public/
```

无需注册、无需配置数据库、无需管理索引。文件放入 library 目录即自动索引，删除即自动清理。

---

## 技术架构

```
┌──────────────┐     ┌──────────────────┐     ┌──────────────┐
│   Vue 3 UI   │ ◄──► │  Node.js API     │ ◄──► │  SQLite FTS5 │
│  (WebView)   │     │  (Express +      │     │  (Prisma)    │
│              │     │   Prisma)         │     │              │
├──────────────┤     ├──────────────────┤     ├──────────────┤
│  Tauri v2    │     │  Rust CLI        │     │  MCP Server  │
│  (桌面壳)     │     │  (omniown)       │     │  (JSON-RPC)  │
│              │     │                  │     │              │
│ 托盘/窗口/    │     │ 文本提取/文件管线/ │     │ AI 客户端直接 │
│ 进程管理      │     │ 文件夹监听        │     │ 查询知识库    │
└──────────────┘     └──────────────────┘     └──────────────┘
```

### 技术栈速览

| 层 | 技术 | 职责 |
|:---|:---|:---|
| 桌面壳 | Tauri v2 | 系统托盘、悬浮面板、子进程管理 |
| 前端 | Vue 3 + TypeScript + Element Plus | 搜索/文档/配置/状态 4 视图 |
| 后端 | Node.js + Express 5 + Prisma 5 | REST API、两阶段 AI 搜索、SSE 推送 |
| 数据库 | SQLite + FTS5 | 元数据存储 + 全文索引 |
| 核心 | Rust (omniown CLI) | 文本提取、文件管线、文件夹监听、MCP Server |
| LLM | OpenAI 兼容接口 | 查询分析 + 策略选择（默认 DeepSeek V4 Flash） |

### 进程模型

| 进程 | 启动方式 | 说明 |
|:---|:---|:---|
| Node.js API | Tauri setup() 自动启动 | Express 服务 port 3001，自动重启 |
| omniown watch | Node.js 启动时 spawn | 递归监听 library 目录 |
| MCP Server | 用户手动触发 | 按需启停，AI 客户端连接 |

---

## 数据流

```
文件放入 library/
  → omniown watch 检测到变更
  → 文本提取（Rust: pdf/docx/xlsx/代码/纯文本）
  → 分类（公开/私密 + 类别 + 风险等级）
  → 写入 SQLite + FTS5 索引
  → SSE 通知前端刷新列表

用户搜索 "我上周的代码文件"
  → Stage 1: LLM 查询分析（改写 + 提取关键词 + 时间范围）
  → Stage 2: LLM 策略选择 → zod 验证
  → 并行执行 3 个策略：recent + category + fulltext
  → 分层合并去重（FTS 结果全保留，非 FTS 最多 5 条补充）
  → 返回 top 20 结果
```

---

## 开发状态

| 维度 | 状态 |
|:---|:---|
| 版本 | v0.1.4 — 早期开发 / 功能打磨过渡期 |
| 核心能力 | 文档管线 ✅ / AI 搜索 ✅ / MCP ✅ / 桌面壳 ✅ |
| 当前焦点 | 流式输出、中文分词优化、跨平台发布 |
| 测试 | Rust 172 单元测试，TypeScript strict 模式 |
| 构建 | Windows Release，Linux/macOS 待恢复 |

---

## 为什么自己做一个而不是用现成的？

| 替代方案 | 不足 |
|:---|:---|
| macOS Spotlight / Everything | 仅文件名，无内容语义搜索 |
| Notion / Obsidian | 生态绑定，非本地优先 |
| 云笔记 (Evernote/语雀) | 数据不在自己硬盘 |
| Elasticsearch | 太重，个人文档杀鸡用牛刀 |
| RAG 方案 (LangChain + Vector DB) | 复杂度高，维护成本大 |

OmniOwn 针对的是**个人文档管理**这个特定场景：不追求万篇级别的扩展性，但求日常使用的低摩擦和可控性。

---

## 项目结构速览

```
omniown/
├── src/          # Rust CLI（文本提取 / 文件管线 / 监听 / MCP）
├── src-tauri/    # Tauri v2 桌面壳
├── server/       # Node.js API（Express / Prisma / AI 搜索）
│   ├── src/
│   │   ├── api/        # 路由层
│   │   ├── services/   # 业务逻辑层
│   │   ├── prompts/    # AI Prompt 模块
│   │   ├── utils/      # 工具（zod 验证等）
│   │   └── db/         # 数据库
│   └── prisma/
├── ui/           # Vue 3 前端
│   └── src/
│       ├── views/      # 4 个页面
│       ├── services/   # API 客户端
│       └── stores/     # 状态管理
└── docs/         # 项目文档
```

---

*OmniOwn —— 你的本地知识库，交给 AI 打理。*
