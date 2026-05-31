# OmniOwn

[![CI](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml/badge.svg)](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml)

**本地优先的个人知识库 — 你的文档，你的硬盘，你的 AI。**

OmniOwn 把你的本地文档变成可搜索的知识库。无需上传、无需联网——文档留在你的硬盘上，AI 搜索通过你的 API Key 直接在本地完成。

---

## ✨ 为什么选择 OmniOwn

| 特性 | OmniOwn | 云端知识库 |
|:---|:---:|:---:|
| 数据隐私 | ✅ 完全本地 | ❌ 上传到云端 |
| 全文搜索 | ✅ FTS5 毫秒级 | ✅ |
| AI 搜索 | ✅ 多策略智能搜索 | ✅ |
| 离线可用 | ✅ 无需网络 | ❌ |
| MCP 协议 | ✅ 可被 AI 客户端直接调用 | ❌ |
| PDF/DOCX/XLSX | ✅ 直接提取文本 | 部分支持 |
| 桌面应用 | ✅ 原生 Tauri（< 50MB） | 网页/Electron |

**独特性：三层架构，各司其职。**

```
Rust CLI  ───→  重型文本提取（PDF/DOCX/XLSX）
Node.js   ───→  业务 API + 8 策略 AI 搜索
Vue 3     ───→  轻量 SPA 前端
Tauri     ───→  原生桌面壳（托盘 + 悬浮面板）
```

**MCP 兼容。** 内置 MCP Server，Claude Desktop / Cursor 等 AI 客户端可直接接入你的本地知识库 — 无需额外配置。

**轻量打包。** Tauri v2 桌面应用 < 50MB，无 Chromium 捆绑，内存占用低。

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

## 🧠 AI 搜索

OmniOwn 的 AI 搜索不是简单的关键词匹配——LLM 分析你的自然语言查询意图，从 **8 种策略** 中选择最优组合：

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
