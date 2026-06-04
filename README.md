# OmniOwn

[![CI](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml/badge.svg)](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml)

**AI 驱动的本地文档管理工具。把文件放入 library，用日常语言描述就能精准找到。**

---

## 为什么需要它

传统的文档管理依赖文件夹 + 文件名。当文档数量增长到成百上千，靠翻目录和记住文件名来查找变得低效。

OmniOwn 换了一种思路：**文件放入 library 目录自动索引，搜索时用自然语言描述，AI 多维度定位。**

完全本地运行，数据不出你的硬盘。

---

## 快速开始

```bash
# 1. 安装依赖
npm --prefix server install
npm --prefix ui install
cargo build

# 2. 启动 API + 前端
npm --prefix server run dev   # → http://127.0.0.1:3001
npm --prefix ui run dev       # → http://127.0.0.1:5173

# 3. 配置 AI（可选 — 启用智能搜索）
# 访问 http://127.0.0.1:5173/#/config 填写 LLM API 信息
```

**使用方式：**

```
将文件放入 library/ 目录
  → 自动检测，提取文本，分类，索引
  → 前端搜索：输入 "我上周的代码文件"
  → AI 多策略匹配 → 返回结果
  → 删除文件 → 自动清理索引
```

---

## Rust CLI

```bash
cargo build

cargo run -- process library/public/note.md   # 手动导入单文件
cargo run -- extract document.pdf             # 提取文本
cargo run -- watch --db-path <path>           # 启动文件夹监听
cargo run -- mcp                              # MCP Server
cargo run -- config-example                   # 输出配置模板
```

---

## Tauri 桌面应用

```bash
npm --prefix server run build
npm --prefix ui run build
cargo tauri build --config src-tauri/tauri.conf.json
# → src-tauri/target/release/bundle/
```

支持 macOS / Windows / Linux，GitHub Actions 自动构建 Release。

---

## MCP 集成

内置 MCP Server，Claude Desktop / Cursor 等 AI 客户端可直接接入：

```json
{
  "mcpServers": {
    "omniown": {
      "command": "omniown",
      "args": ["mcp"]
    }
  }
}
```

---

## 搜索策略

| 策略 | 说明 | 示例 |
|:---|:---|:---|
| `fulltext` | FTS5 全文搜索 | "rust async" |
| `category` | 按分类筛选 | "代码" |
| `filetype` | 按文件类型 | "PDF" |
| `recent` | 时间范围 | "本周" |
| `privacy` | 公开/私密 | "私有文件" |
| `filename` | 文件名匹配 | "README" |
| `tag` | 标签检索 | "bugfix" |

## 质量

```bash
cargo test                                    # 172 tests
cargo clippy -- -D warnings
npm --prefix server run build                 # tsc
npm --prefix ui run build                     # vue-tsc + vite
```

---

## 技术栈

| 层 | 技术 |
|:---|:---|
| 文本提取 | Rust + lopdf + calamine + quick-xml |
| 文件监听 | Rust + notify (inotify / FSEvents / ReadDirectoryChanges) |
| 全文检索 | SQLite FTS5 |
| API | Express + Prisma + TypeScript |
| 前端 | Vue 3 + Pinia + Vite |
| 桌面 | Tauri v2 (tray + shell + dialog + positioner) |
| 配置 | TOML |

---

## 项目结构

```
omniown/
├── src/                  # Rust CLI (process / extract / watch / mcp)
├── server/               # Node.js/TS API (Express + Prisma + SQLite)
├── ui/                   # Vue 3 前端 (Search / Documents / Config / Status)
├── src-tauri/            # Tauri v2 桌面壳
├── docs/                 # 架构 / CLI / 配置 / 数据库 / 问题追踪
└── .github/workflows/    # CI + Release
```

---

## License

MIT © 2026 zj-zhuo
