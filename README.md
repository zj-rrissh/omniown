# OmniOwn

[English](README.md) | [中文](README.zh-CN.md)

OmniOwn is an AI-powered local document management tool. Put files into your library folder, let OmniOwn index them automatically, and use natural-language queries to find exactly what you need. It runs locally and keeps your data on your own disk.

---

## Features

- **Library folder management**: files placed in the library are indexed automatically with text extraction, classification, and SQLite FTS5 search.
- **Real-time folder watching**: `omniown watch` uses the Rust `notify` crate to recursively watch the library folder and keep the database in sync.
- **AI multi-strategy search**: natural-language queries can be mapped to multiple search strategies across full text, category, file type, time, privacy signals, filename, tags, and summary.
- **FTS5 full-text search**: SQLite FTS5 provides fast local content search.
- **Tauri v2 desktop app**: a lightweight desktop shell with a floating panel and system tray support.
- **MCP server**: built-in MCP support lets Claude Desktop, Cursor, and other compatible AI clients query the local knowledge base.
- **Configurable paths**: choose any folder as the library from the settings page.

---

## Tech Stack

| Layer | Technology |
|:---|:---|
| Text extraction | Rust + lopdf + calamine + quick-xml |
| File watching | Rust + notify |
| Full-text search | SQLite FTS5 + Prisma ORM v5 |
| API | Node.js + Express + TypeScript |
| Frontend | Vue 3 + Pinia + Vite |
| Desktop | Tauri v2 |
| CI/CD | GitHub Actions |

---

## Installation

- **Desktop app**: download the Windows installer from [Releases](https://github.com/zj-rrissh/omniown/releases). Windows is the current release target.
- **Development mode**: clone the repository, install dependencies, build the server and frontend, then run the Tauri app.
- **CLI**: install from the repository with `cargo install --path .`.

```bash
git clone https://github.com/zj-rrissh/omniown.git
cd omniown

npm --prefix server install
npm --prefix ui install
npm --prefix server run build
npm --prefix ui run build
cargo build
```

---

## Documentation

[Architecture](docs/architecture.md) · [CLI](docs/cli.md) · [Configuration](docs/config.md) · [Database](docs/database.md) · [Development](docs/development.md) · [Troubleshooting](docs/troubleshooting.md) · [Git History](docs/git-history.md)

---

## Quality

| Check | Status |
|:---|:---|
| Rust unit tests | 172 |
| TypeScript strict mode | Enabled |
| Clippy | Zero warnings target |
| Rustfmt | Enforced |
| Windows release build | Supported |
| macOS / Linux release build | Paused |

---

See [CHANGELOG](CHANGELOG.md), [Git History](docs/git-history.md), or the [commit history](https://github.com/zj-rrissh/omniown/commits/main) for project evolution.
