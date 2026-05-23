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
4. 启动空闲 Embedding Worker
5. 开始监听 `inbox/`
6. 按 `Ctrl+C` 退出

**注意事项：**

- 这是唯一一个进入事件循环的命令
- 其他命令执行完即退出

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
Path: library/public/2026-05-22_a81f39c2_rust_note.md
Type: public / notes
Snippet: ...学习 [rust] 异步编程...
Rank: -2.35

共找到 1 个结果。
```

**注意事项：**

- 依赖 FTS5 索引
- 无匹配时输出"没有找到匹配的文档"

---

## `cargo run -- semantic-search <query> [--provider <name>] [--folder <type>] [--limit <n>]`

基于 embedding 的向量语义搜索。

```bash
# 使用默认 mock provider
cargo run -- semantic-search "rust 异步队列"

# 指定 provider
cargo run -- semantic-search "rust 异步队列" --provider mock

# 限制搜索范围
cargo run -- semantic-search "rust 异步队列" --folder public --limit 10
```

**参数：**

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `--provider` | embedding provider 名称 | config 中的 provider |
| `--folder` | 限定 folder_type（public/private） | 不限定 |
| `--limit` | 最大返回条数 | config 中的 default_limit |
| `--dim` | 向量维度 | config 中的 dim |

**注意事项：**

- 依赖 `document_embeddings` 表中的向量数据
- 必须先运行 `cargo run -- embed` 生成 embedding
- 如无可用 embedding，输出提示：`没有可搜索的 embedding。请先运行：cargo run -- embed`

---

## `cargo run -- embed [--provider <name>] [--limit <n>] [--dim <n>]`

批量计算文档的 embedding 向量。

```bash
# 使用默认 mock provider 处理待计算文档
cargo run -- embed --provider mock

# 限制处理数量
cargo run -- embed --provider mock --limit 10
```

**参数：**

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `--provider` | embedding provider 名称 | config 中的 provider |
| `--limit` | 最大处理文档数 | config 中的 batch_size |
| `--dim` | 向量维度 | config 中的 dim |

**输出示例：**

```
🧠 embedded: [note_001.md] 1
🧠 embedded: [note_002.md] 2
✅ embedding completed: done=2 skipped=0 failed=0
```

**注意事项：**

- 只处理 `processing_status = 'indexed'` 且 `content` 非空的文档
- 对于当前 provider 已存在 embedding 的文档自动跳过
- 默认构建下 local provider 是 stub，执行会清晰报错：

  ```
  ❌ LocalEmbeddingProvider is experimental and not enabled yet. Build with --features local-embedding or use --provider mock.
  ```

- 开启 `local-embedding` feature 后可运行本地实验 provider：

  ```bash
  cargo run --features local-embedding -- embed --provider local
  cargo run --features local-embedding -- semantic-search "rust async queue" --provider local
  ```

---

## `cargo run -- embedding-provider-info [--provider <name>] [--dim <n>]`

查看 embedding provider 信息。

```bash
# 列出所有可用 provider
cargo run -- embedding-provider-info

# 查看特定 provider
cargo run -- embedding-provider-info --provider mock
```

**输出示例：**

```
Provider: mock
  Status: available
  Model name: mock-hash-384
  Dim: 384
  Functional: yes
  Network: no
  Purpose: tests, fallback, deterministic local development
```

---

## `cargo run -- config-example`

输出配置模板到标准输出。

```bash
cargo run -- config-example > config/config.toml
```

输出完整的 TOML 配置示例，包含所有默认值。

---

## `cargo run -- doctor`

系统健康检查。

```bash
cargo run -- doctor
```

**检查项：**

1. 目录路径（是否存在）
2. 数据库（可打开、schema 版本、pending migration、主键结构）
3. Embedding Provider（可用性、功能性）
4. Worker 配置
5. 搜索配置

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

Provider:  mock
Embeddings:
  total:    42
  current_model: mock-hash-384
  current_model_embeddings: 40
  pending_for_current_model: 60

Worker:    enabled
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
