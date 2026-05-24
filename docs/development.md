# 开发文档

## 常用命令

```bash
# 代码格式化
cargo fmt

# 运行全部测试
cargo test

# Clippy 检查（零警告策略）
cargo clippy -- -D warnings

# 系统健康检查
cargo run -- doctor

# 状态概览
cargo run -- status

# 本地只读 Web UI（生产模式）
cd ui
npm install
npm run build
cd ..
cargo run -- serve

# 本地只读 Web UI（开发模式，两个终端）
cargo run -- serve
cd ui && npm run dev

# 数据库迁移
cargo run -- migrate

# 启动哨兵
cargo run
```

---

## 开发原则

### 架构原则

1. **先 CLI，后 UI** — 所有功能先在 CLI 中可用，UI 是对 CLI 的补充
2. **先 SQLite，后复杂检索** — 不做向量数据库，不引入外部搜索引擎
3. **默认离线** — 所有核心功能不依赖网络
4. **不阻塞导入** — 不因为 embedding 失败导致文件导入失败
5. **固定运行根目录** — 后端命令默认从项目根目录运行；如需从其他目录启动必须显式设置 `OMNIOWN_ROOT`

### 数据库原则

6. **新功能必须有测试** — 包括单元测试和集成测试
7. **Migration 保护旧数据** — 不破坏现有数据，幂等可重复执行
8. **涉及表结构变更时必须有事务保护** — 避免 DROP 后失败导致数据丢失
9. **不直接修改旧 migration** — 新 schema 变更必须新增 migration

---

## Migration 规则

### 新增 migration 步骤

1. 在 `src/migration.rs` 的 `MIGRATIONS` 数组末尾追加新条目
2. 实现 `fn migration_N_xxx(conn: &Connection) -> rusqlite::Result<()>`
3. 添加对应测试

### 约束

- `version` 必须递增（当前最大 +1）
- 迁移函数必须是幂等的（使用 `IF NOT EXISTS` / `INSERT OR IGNORE` 等）
- 涉及 DROP + CREATE 的迁移必须使用事务保护
- 测试必须覆盖旧数据迁移场景

### 当前迁移版本

详见 [database.md](./database.md#migrations-版本列表)。

---

## CI

项目使用 GitHub Actions 进行持续集成，配置文件在 `.github/workflows/ci.yml`。

触发条件：
- `push` / `pull_request` 到 `main` / `master` 分支

检查内容：
1. `cargo fmt -- --check` — 代码格式化
2. `cargo test` — 全部测试
3. `cargo clippy -- -D warnings` — 零警告

CI badge: `[![CI](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml/badge.svg)](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml)`

---

## 测试策略

- 使用 `Connection::open_in_memory()` 创建临时数据库，不依赖文件系统
- `setup_db()` 运行完整迁移链，确保测试环境与生产一致
- 集成测试使用临时目录（`std::env::temp_dir()`）
- 每个测试独立清理

### 运行测试

```bash
# 全部测试
cargo test

# 特定测试
cargo test migration_5_converts
cargo test doctor_reports_schema_version

# 查看测试列表
cargo test -- --list
```

---

## 下一步 Roadmap

| 序号 | 任务 | 说明 |
|------|------|------|
| Task 15 | GitHub Actions CI | 自动化 test/clippy/fmt |
| Task 16 | Embedding 剥离 | ✅ 已移除，`ai-search` 替代语义搜索 |
| Task 17 | 更丰富的文本提取 | ✅ 已完成：extractor + Markdown/HTML/文本类扩展 |
| Task 18 | 极简前端 | ✅ 已完成：Vue + TypeScript Web UI + JSON API |
| Task 19 | Tauri 桌面应用 | ✅ Phase 1-6 完成：托盘面板 + sidecar + 路由 |
| Task 20 | 保留文件名 | ✅ 导入到 library 时保留原文件名 |

> **注意：** 以上 roadmap 是方向性规划，具体内容和优先级可能根据项目实际需要调整。
