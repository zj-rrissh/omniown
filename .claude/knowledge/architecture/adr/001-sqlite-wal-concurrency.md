# ADR 001: SQLite WAL 模式并发访问

**日期**：2026-05  
**状态**：已采纳

## 背景

OmniOwn 有两套代码同时访问同一个 SQLite 数据库文件：
- **Node.js**：通过 Prisma ORM（DELETE journal 模式）
- **Rust CLI (omniown watch)**：通过 rusqlite

Prisma 默认使用 DELETE journal 模式，rusqlite 在 `db::init_database()` 中切换到 WAL 模式。当两个进程以不同 journal 模式访问同一文件时，SQLite 会报 "disk I/O error" 并损坏数据。

## 决策

**统一使用 WAL 模式**。Node.js 启动时在 `prisma db push` 之后立即执行 `PRAGMA journal_mode=WAL`，确保在 Rust CLI 访问前已切换到 WAL。

## 实施方案

1. Node.js `index.ts`：`execSync('sqlite3 "..." "PRAGMA journal_mode=WAL"')`
2. Rust `db.rs`：`conn.execute_batch("PRAGMA journal_mode=WAL")` 作为幂等保护
3. 数据库路径统一为绝对路径（避免相对路径歧义）

## 后果

- 会产生 `dev.db-wal` 和 `dev.db-shm` 文件（.gitignore 已覆盖）
- 两个进程可安全并发访问
