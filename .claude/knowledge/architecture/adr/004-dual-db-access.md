# ADR 004: Prisma ORM + rusqlite 双数据库访问

**日期**：2026-05  
**状态**：已采纳

## 背景

Node.js API 通过 Prisma ORM 访问 SQLite，Rust CLI (omniown watch) 需要实时写入索引结果到同一数据库。两个运行时两套数据库驱动必须安全共存。

## 决策

**Prisma ORM 用于 Node.js CRUD，rusqlite 用于 CLI 实时写入**，通过 WAL 模式实现并发安全（见 ADR 001）。

- Prisma：类型安全、迁移管理、关系查询
- rusqlite：零成本 FFI、无 Node.js 依赖、CLI 端直接操作

## 约束

1. 两套代码必须操作同一数据库文件（通过 `--db-path` CLI arg 传递绝对路径）
2. Schema 以 `schema.prisma` 为准，rusqlite 端手动同步表结构
3. FTS5 虚拟表/触发器由 Node.js 端管理（Prisma 不支持）
4. Rust 端 `db.rs` 函数需要与 Prisma schema 字段名保持一致

## 后果

- Schema 变更需要确认两套代码同步
- 新增了 `troubleshooting.md` 问题 1（split-brain 数据库）和问题 2（WAL 损坏）的教训
