# OmniOwn

[![CI](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml/badge.svg)](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml)

**AI 驱动的本地文档管理工具 — 丢进去，说出来，找到它。**

传统的文档管理器在文件少时还好用，但随着文档堆积到成百上千个，靠翻文件夹和记文件名来找文档就变成了体力活。OmniOwn 换了一种思路：你只管把文档丢进去，将来用**日常口语描述**你要找的内容，AI 帮你定位。

比如你半年前存了一份 PDF，只记得「那个讲微服务的架构图」，不用回忆文件名、不用翻目录——在搜索框里打「那个讲微服务的架构图」，AI 从 8 个维度同时入手，几毫秒内给你答案。

---

## ✨ 它和别人有什么不同

传统的文档管理 = 文件夹 + 文件名 + 肉眼翻找。OmniOwn = **口语化描述 → AI 多维度理解 → 精准定位**。

| 你会遇到的问题 | 传统方式 | OmniOwn |
|:---|:---|:---|
| 「半年前那个讲 Rust 并发的笔记」 | 翻文件夹、猜文件名、逐个打开 | 打字描述，AI 直接找到 |
| 「上个月下载的那份合同 PDF」 | 按时间排序、肉眼扫 | 说「上个月的合同」，按时间+文件类型定位 |
| 「叫什么名字忘了，内容是关于 API 设计的」 | 几乎无解 | 全文检索 + AI 语义理解 |
| 文档全在本地，不想上传到任何云端 | — | ✅ 完全本地，数据不出你的硬盘 |

**只要放入文档，未来口语化描述就能找到。** 不需要分类、不需要打标签、不需要记住文件名——把文件丢进 `inbox/`，剩下的交给 OmniOwn。

---

## 🏗️ 它是怎么做到的

```
Rust CLI  ───→  提取 PDF/DOCX/XLSX 文本，自动分类
Node.js   ───→  8 策略 AI 搜索引擎，并行匹配
Vue 3     ───→  轻量搜索界面，口语化输入
Tauri     ───→  原生桌面应用（托盘 + 悬浮面板）
```

**MCP 兼容。** 内置 MCP Server，Claude Desktop / Cursor 等 AI 客户端可直接接入你的文档库——无需额外配置。

**轻量。** Tauri v2 桌面应用 < 50MB，不捆绑浏览器内核。

---

## 🚀 快速开始

### 方式一：Web 开发模式

```bash
# 终端 1 — Node.js API（port 3001）
npm --prefix server run dev

# 终端 2 — Vue 开发服务器（port 5173）
npm --prefix ui run dev
```

### 方式二：Rust CLI

```bash
cargo build

# 导入文件
cargo run -- process inbox/note.md

# 提取文本
cargo run -- extract document.pdf

# MCP Server（AI 客户端可连接）
cargo run -- mcp

# 生成配置模板
cargo run -- config-example
```

### 方式三：Tauri 桌面

```bash
npm --prefix server run build
npm --prefix ui run build
cargo tauri build --manifest-path src-tauri/Cargo.toml
# → src-tauri/target/release/bundle/
```

---

## 🧠 口语化搜索，不是关键词匹配

传统的文档搜索靠关键词——你得猜文档里有什么词。OmniOwn 的 AI 搜索理解你的**自然语言意图**，从 8 个维度同时定位：

| 策略 | 适用场景 | 示例 |
|:---|:---|:---|
| `fulltext` | 关键词全文搜索 | "rust async" |
| `category` | 按分类筛选 | "代码相关的文档" |
| `filetype` | 按文件类型 | "所有 PDF 文件" |
| `summary` | 按摘要搜索 | "关于性能优化的记录" |
| `recent` | 时间范围 | "本周的文件" |
| `privacy` | 公开/私密 | "私有文档" |
| `filename` | 文件名匹配 | "名叫 README 的文件" |
| `tag` | 标签检索 | "带 bugfix 标签的" |

查询 `?q=我上周的代码文件&ai=true` → LLM 拆为 `recent` + `category` → 并行执行 → 合并去重 → 返回 Top 20。

---

## 🏗️ 项目结构

```
omniown/
├── src/                       # Rust CLI
│   ├── main.rs                # 入口：process / extract / mcp
│   ├── extractor.rs           # 文本提取（PDF/DOCX/XLSX/MD/HTML）
│   ├── processor.rs           # 文件导入管线（分类+存储+DB写入）
│   ├── mcp.rs                 # MCP Server（SQLite FTS5 直连）
│   ├── db.rs                  # SQLite CRUD + FTS5
│   ├── config.rs              # TOML 配置
│   └── fs_layout.rs           # 目录规划
│
├── server/                    # Node.js/TS API
│   └── src/
│       ├── api/               # status / documents / search / config
│       ├── services/          # search / ai / import
│       └── db/                # Prisma + FTS5 初始化
│
├── ui/                        # Vue 3 + Pinia 前端
│   └── src/
│       ├── views/             # Search / Documents / Config / Status
│       ├── stores/            # Pinia stores
│       └── services/          # HTTP client 层
│
├── src-tauri/                 # Tauri v2 桌面壳
├── docs/                      # 架构 / CLI / 配置 / 数据库
├── .github/workflows/         # CI + Release
└── config.example.toml        # 配置模板
```

---

## 🔍 质量检查

```bash
cargo test                                    # 172 tests
cargo clippy -- -D warnings                   # 零警告
npm --prefix server run build                 # tsc
npm --prefix ui run build                     # vue-tsc + vite
cargo test --manifest-path src-tauri/Cargo.toml  # 13 tests
```

---

## 🛠️ 技术栈

| 层 | 技术 |
|:---|:---|
| 文本提取 | Rust + lopdf + calamine + quick-xml |
| 全文检索 | SQLite FTS5（虚拟表 + 触发器自动同步） |
| API | Express 5 + Prisma 5 + TypeScript strict |
| 前端 | Vue 3 + Pinia + Vite 6 |
| 桌面 | Tauri v2（tray + shell + positioner） |
| 配置 | TOML（@iarna/toml + serde） |
| CI/CD | GitHub Actions（fmt → test → clippy） |
