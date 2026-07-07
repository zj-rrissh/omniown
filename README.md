# OmniOwn

[English](README.md) | [中文](README.zh-CN.md)

**AI-Native Local Document Search Engine** — "AI plans the search, the engine executes it." OmniOwn doesn't store knowledge or answer questions directly; instead, it works as an AI-powered search brain that understands your query, plans the retrieval, retries on failure, and explains the results. **All data stays on your local disk — zero cloud dependency.**

---

## Architecture Overview

OmniOwn uses a **three-layer architecture** where Rust handles heavy lifting, Node.js orchestrates business logic, and Vue provides the UI:

```
┌──────────────────────────────────────────────────────────────┐
│  Tauri v2 Desktop Shell (src-tauri/)                         │
│  • System tray + floating panel  • Sidecar process manager   │
│  • Auto-spawns Node.js API on startup                        │
└─────────────────────┬────────────────────────────────────────┘
                      │ WebView
┌─────────────────────▼────────────────────────────────────────┐
│  Vue 3 + TypeScript (ui/)                                    │
│  • SearchView / DocumentsView / ConfigView / StatusView      │
│  • Pinia stores · Element Plus · Hash routing                │
└─────────────────────┬────────────────────────────────────────┘
                      │ HTTP (localhost:3001)
┌─────────────────────▼────────────────────────────────────────┐
│  Node.js + Express + TypeScript API (server/)                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  AI Search Pipeline                 FTS5 Search Engine  │ │
│  │  ┌──────────┐ ┌──────────────┐     • SQLite FTS5        │ │
│  │  │ Query    │ │ Strategy     │     • BM25 ranking       │ │
│  │  │ Analysis │→│ Selection    │     • Snippet highlight  │ │
│  │  │ (LLM)    │ │ (LLM + zod)  │     • 8 search dims     │ │
│  │  └──────────┘ └──────┬───────┘                          │ │
│  │                      │ parallel execution                │ │
│  │                      ▼                                   │ │
│  │  ┌─────────────────────────────────────────────────────┐ │ │
│  │  │  Prisma ORM → SQLite (FTS5) · Config · Watch mgr   │ │ │
│  │  └─────────────────────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────┬────────────────────────────────────────┘
                      │ child_process (spawn / stdio)
┌─────────────────────▼────────────────────────────────────────┐
│  Rust Core + CLI (src/)  —  172 unit tests, zero Clippy      │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  extractor   processor    watch (notify)    MCP Server  │ │
│  │  • 10+       • Pipeline   • Recursive      • JSON-RPC   │ │
│  │    formats     engine       listening        2.0        │ │
│  │  • PDF/XLSX  • Classify   • 1s debounce    • Tool call  │ │
│  │  • Office    • Metadata   • Stability       protocol    │ │
│  │  • Code      • Persist      detection                   │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

---

## Key Engineering Highlights

### AI Search Pipeline (LLM + FTS5 Hybrid)

A two-stage pipeline that turns "AI search brain" into a deterministic execution engine:

```
User: "my code files from last week"
  │
  ▼ Stage 1 — Query Analysis (LLM)
  │   rewrite + keyword extraction + intent classification
  │   → {rewrittenQuery, keywords, intent, suggestedCategory, timeRangeDays}
  │   ↓ fallback: raw query on LLM failure
  ▼ Stage 2 — Strategy Selection (LLM + zod JSON Schema)
  │   LLM picks optimal strategy combination + validates via zod schema
  │   → [{strategy: "recent", params: {days: 7}}, {strategy: "fulltext", params: {query: "code"}}, ...]
  │   ↓ fallback: default fulltext search on LLM failure
  ▼ Parallel Execution (Promise.allSettled)
  │   8 strategies run concurrently across dimensions
  │   → fulltext / category / filetype / summary / recent / privacy / filename / tag
  ▼ Tiered Merge & Dedup
  │   • FTS hits (rank ≠ -1): all retained
  │   • Non-FTS hits (rank = -1): max 5 supplement
  │   • Pure non-FTS (browsing): unlimited
  ▼ Top 20 → Response
```

**Key design decisions:**
- **Planner → Executor separation**: LLM only outputs a `SearchPlan` (strategy list + confidence), never touches search execution. New strategies added by implementing one interface.
- **Prompt variants (v1/v2)**: Modular prompt system supports A/B testing without code changes. v2 adds few-shot examples + doc stats context.
- **60s TTL cache**: `getDocumentStats()` cached to avoid redundant DB queries per AI search; cache invalidated on file change events.
- **Graceful degradation**: AI unavailable → falls back to plain FTS5 search. System works independently of LLM.

### Real-time Indexing Pipeline (Rust)

```
notify (FS event)
  │
  ▼ 1s silence + file size unchanged
  │   Stability detection prevents premature indexing of in-progress writes
  ▼ Dedup fingerprint (30s window)
  │   Prevents duplicate processing of repeated events
  ▼ extract() → classify() → persist()
  │   PipelineStep trait: new parsers insert into step chain, no core changes
  ▼ Error isolation → quarantine/
  │   Failed files isolated, SSE push to frontend
```

### MCP Protocol Integration

Built-in MCP server exposes the knowledge base to AI clients (Claude Desktop, Cursor, etc.) via standard JSON-RPC 2.0 protocol. Tools available remotely without any additional configuration.

---

## Tech Stack

| Layer | Technology | Highlights |
|:------|:-----------|:-----------|
| Text Extraction | Rust · lopdf · calamine · quick-xml | 10+ formats, single binary |
| File Watching | Rust · notify (inotify/FSEvents/ReadDirectoryChanges) | Cross-platform, 1s debounce |
| Full-text Search | SQLite FTS5 · Prisma ORM v5 | BM25 ranking, snippet highlight, zero external deps |
| AI Orchestration | Node.js · Express · TypeScript | Two-stage LLM pipeline, zod validation, prompt variants |
| Frontend | Vue 3 · Pinia · Vite · Element Plus | Floating panel, hash routing |
| Desktop | Tauri v2 (tray + shell + dialog + positioner) | ~5 MB binary, sidecar process mgmt |
| Protocol | MCP (Model Context Protocol) | JSON-RPC 2.0, tool-call pattern |
| CI/CD | GitHub Actions | Auto-build Windows installer per release |

---

## Quality Metrics

| Metric | Value |
|:-------|:------|
| Rust unit tests | 172 |
| TypeScript strict mode | Enabled |
| Clippy | Zero warnings target |
| Rustfmt | Enforced |
| Windows release build | Supported |

---

## Quick Start

```bash
git clone https://github.com/zj-rrissh/omniown.git
cd omniown

npm --prefix server install
npm --prefix ui install
npm --prefix server run build
npm --prefix ui run build
cargo build
```

Or download the Windows installer from [Releases](https://github.com/zj-rrissh/omniown/releases).

---

## Documentation

[Architecture](docs/architecture.md) · [CLI](docs/cli.md) · [Configuration](docs/config.md) · [Database](docs/database.md) · [Development](docs/development.md) · [Git History](docs/git-history.md) · [CHANGELOG](CHANGELOG.md)
