# CLI 命令

所有命令通过 `cargo run -- <command>` 执行。编译后也可直接运行二进制 `./target/debug/omniown <command>`。

---

## `cargo run`

启动哨兵模式：监控 `inbox` 目录，自动导入并处理新文件。

```bash
cargo run
```

**预期行为：**

1. 初始化目录结构
2. 初始化/迁移数据库
3. 输出状态概览
4. 开始监听 `inbox/`
5. 按 `Ctrl+C` 退出

**注意事项：**

- 这是唯一一个进入事件循环的命令
- 其他命令执行完即退出
- 只会导入 extractor 支持的 UTF-8 文本类文件：

  ```text
  txt, md, markdown, html, htm,
  rs, js, ts, jsx, tsx, py, java, go, cpp, c, h, hpp, css, sh, sql,
  json, toml, yaml, yml, csv, log
  ```

- Markdown / HTML 会先提取正文再写入全文索引；不支持的扩展名会跳过，无法按 UTF-8 读取的文件会进入 `quarantine/`

---

## `cargo run -- search <query>`

FTS5 全文搜索。

```bash
cargo run -- search rust
cargo run -- search "async queue"
```

**输出示例：**

```
[1] rust_note.md
Path: library/public/rust_note.md
Type: public / notes
Snippet: ...学习 [rust] 异步编程...
Rank: -2.35

共找到 1 个结果。
```

**注意事项：**

- 依赖 FTS5 索引
- 无匹配时输出"没有找到匹配的文档"

---

## `cargo run -- ai-search <query>`

AI 驱动的自然语言搜索。

```bash
cargo run -- ai-search "rust 异步队列的最佳实践"
```

**原理：** LLM 将自然语言问题转化为搜索词 → FTS5 全文搜索。

**前提：** 需要在 `config/omniown.toml` 的 `[ai]` 节配置 API key。

---

## `cargo run -- config-example`

输出配置模板到标准输出。

```bash
cargo run -- config-example > config/config.toml
```

输出完整的 TOML 配置示例，包含所有默认值。

---

## 启动本地服务

API 服务现已改为 Node.js，不再通过 Rust 启动。

生产模式：

```bash
# 后端
cd server && npm install && npm run build && node dist/index.js

# 前端
cd ui && npm install && npm run build
```

开发模式：

```bash
# Terminal 1: API
cd server && npm run dev   # → http://127.0.0.1:3001

# Terminal 2: 前端
cd ui && npm run dev       # → http://localhost:5173
```

**UI 能力：**

1. 状态概览：schema、文档统计
2. 文档列表：文件名、路径、folder、category、risk、更新时间
3. 全文搜索：FTS5 + LLM 智能搜索
4. 文档详情：元数据 + 文本内容
5. AI 配置：LLM API 设置
6. 配置读写：通过 /api/config 管理

**本地 API：**

| 路径 | 说明 |
|------|------|
| `GET /api/status` | 状态概览 |
| `GET /api/documents` | 文档列表 |
| `GET /api/search?q=` | 全文搜索 |
| `GET /api/documents/:id` | 文档详情 |
| `GET /api/config` | 读取配置 |
| `PUT /api/config` | 更新配置 |

---

## `cargo run -- doctor`

系统健康检查。

```bash
cargo run -- doctor
```

**检查项：**

1. 目录路径（是否存在）
2. 数据库（可打开、schema 版本、pending migration）
3. 搜索配置

---

## `cargo run -- status`

系统状态概览。

```bash
cargo run -- status
```

**输出示例：**

```
OmniOwn Status

Database: ./index/omniown.db
Root: .

Schema:
  current_version: 5
  pending_migrations: 0

Documents:
  total:    120
  public:   70
  private:  30
  indexed:  100
  failed:   0
```

---

## `cargo run -- migrate`

手动执行数据库迁移。

```bash
cargo run -- migrate
```

**输出示例：**

```
OmniOwn Migration

Applied:
  - 5 document_embeddings_composite_primary_key

Skipped:
  - 1 create_documents
  - 2 create_documents_fts
  - 3 create_document_embeddings
  - 4 create_indexes

Current schema version: 5
```

**注意事项：**

- 迁移自动在启动哨兵或首次数据库访问时执行
- 本命令用于主动触发或验证迁移状态
- 迁移是幂等的：重复执行 safety

---

## `cargo run -- cleanup-old-library`

删除旧版 `library/{public|private}/YYYY-MM-DD_hash8_filename` 命名格式的残留文件，
并同步删除数据库中指向这些旧路径的文档记录。

```bash
cargo run -- cleanup-old-library
```

新导入的文件会保留原始文件名，落到：

```text
library/{public|private}/{safe_filename}
```

同目录存在同名文件时，交互终端会提示覆盖或取消；非交互环境默认取消并记录失败。
