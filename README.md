## OmniOwn

OmniOwn 是一个 AI 驱动的本地文档管理工具。把文件放入 library 目录，自动索引；用日常语言描述，AI 帮你精准定位。**完全本地运行，数据不出你的硬盘。**

---

## ✨ 核心功能

- **📁 library 目录管理** — 文件放入 library 自动索引（文本提取 + 分类 + FTS5 全文检索），删除文件自动清理索引
- **🔄 实时文件夹监听** — `omniown watch` 基于 notify crate 跨平台递归监听 library 目录，增删自动同步数据库
- **🧠 AI 多策略搜索** — 自然语言查询 → LLM 选择策略 → 并行执行 8 个搜索维度（全文/分类/文件类型/时间/隐私/文件名/标签/摘要）
- **🔍 FTS5 全文检索** — SQLite FTS5 虚拟表，文件内容全文搜索，毫秒级响应
- **🖥️ Tauri v2 桌面应用** — 跨平台（macOS / Windows / Linux），透明悬浮面板 + 系统托盘
- **🔌 MCP Server** — 内置 MCP 协议支持，Claude Desktop / Cursor 等 AI 客户端可直接接入知识库
- **⚙️ 可配置路径** — 设置页面选择任意目录作为 library，支持系统目录选择器

---

## 🏗️ 技术栈

| 层 | 技术 |
|:---|:---|
| 文本提取 | Rust + lopdf + calamine + quick-xml |
| 文件监听 | Rust + notify (inotify / FSEvents / ReadDirectoryChanges) |
| 全文检索 | SQLite FTS5 + Prisma ORM v5 |
| API | Node.js + Express + TypeScript |
| 前端 | Vue 3 + Pinia + Vite |
| 桌面 | Tauri v2 (tray + shell + dialog + positioner) |
| CI/CD | GitHub Actions (fmt → clippy → test → release) |

---

## 📦 安装

- **桌面应用**: 从 [Releases](https://github.com/zj-rrissh/omniown/releases) 下载对应平台的 `.dmg` / `.exe` / `.AppImage`
- **开发模式**: `git clone` → `npm install` → `cargo build` → `npm run dev`
- **CLI**: `cargo install --path .`

---

## 📚 文档

[架构](docs/architecture.md) · [CLI](docs/cli.md) · [配置](docs/config.md) · [数据库](docs/database.md) · [开发](docs/development.md) · [问题追踪](docs/troubleshooting.md)

---

## 📊 质量

| 指标 | 数值 |
|:---|:---|
| Rust 单元测试 | 172 |
| TypeScript 严格模式 | ✅ |
| Clippy 零警告 | ✅ |
| Rustfmt | ✅ |
| 三平台构建 | ✅ |

---

## 🔜 后续计划

- 前端文档详情页「打开文件」按钮
- Windows 端到端测试验证
- i18n 多语言支持

---

📝 详细变更历史请查看 [Commits](https://github.com/zj-rrissh/omniown/commits/main)
