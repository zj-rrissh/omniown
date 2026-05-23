# 配置文档

## 配置加载顺序

```
1. 内置默认值（hardcoded in src/config.rs）
2. OmniOwn/config/config.toml（文件配置）
3. 环境变量覆盖
```

当配置文件中某字段缺失时，自动使用内置默认值。

---

## 默认配置

以下是通过 `cargo run -- config-example` 输出的完整默认配置：

```toml
[paths]
root = "."
inbox = "inbox"
library = "library"
index = "index"
cache = "cache"
logs = "logs"
quarantine = "quarantine"
trash = "trash"
config_dir = "config"
database = "index/omniown.db"

[embedding]
provider = "mock"
dim = 384
max_chars_per_doc = 100000

[worker]
enabled = true
idle_interval_ms = 60000
batch_size = 4
max_docs_per_cycle = 100

[search]
default_limit = 20
fts_enabled = true
semantic_enabled = true
```

### 配置段说明

**`[paths]`** — 目录路径配置

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `root` | `"."` | 项目根目录（所有相对路径的基础） |
| `inbox` | `"inbox"` | 监控目录 |
| `library` | `"library"` | 文件存储根目录 |
| `index` | `"index"` | 数据库目录 |
| `database` | `"index/omniown.db"` | SQLite 数据库路径 |
| `config_dir` | `"config"` | 配置文件目录 |
| `cache` / `logs` / `quarantine` / `trash` | 同上 | 运行时目录 |

**`[embedding]`** — Embedding 配置

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `provider` | `"mock"` | Embedding provider 名称（mock / local） |
| `dim` | `384` | 向量维度 |
| `max_chars_per_doc` | `100000` | 单文档最大处理字符数 |

`local` provider 默认是 stub；需要用 `cargo run --features local-embedding -- ...`
启用离线 token-hash 实验实现。

**`[worker]`** — 空闲 Embedding Worker 配置

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `enabled` | `true` | 是否启用空闲 worker |
| `idle_interval_ms` | `60000` | 轮询间隔（毫秒） |
| `batch_size` | `4` | 每批处理文档数 |
| `max_docs_per_cycle` | `100` | 每周期最大处理数 |

**`[search]`** — 搜索配置

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `default_limit` | `20` | 默认搜索结果条数 |
| `fts_enabled` | `true` | 启用全文搜索 |
| `semantic_enabled` | `true` | 启用语义搜索 |

---

## 环境变量

环境变量优先级高于配置文件。

| 变量 | 覆盖配置字段 | 示例 |
|------|-------------|------|
| `OMNIOWN_ROOT` | `paths.root` | `OMNIOWN_ROOT=/data/notes` |
| `OMNIOWN_DB_PATH` | `paths.database` | `OMNIOWN_DB_PATH=/custom/db.sqlite` |
| `OMNIOWN_EMBEDDING_PROVIDER` | `embedding.provider` | `OMNIOWN_EMBEDDING_PROVIDER=local` |
| `OMNIOWN_EMBEDDING_DIM` | `embedding.dim` | `OMNIOWN_EMBEDDING_DIM=768` |
| `OMNIOWN_WORKER_ENABLED` | `worker.enabled` | `OMNIOWN_WORKER_ENABLED=false` |
| `OMNIOWN_WORKER_BATCH_SIZE` | `worker.batch_size` | `OMNIOWN_WORKER_BATCH_SIZE=10` |
| `OMNIOWN_WORKER_IDLE_INTERVAL_MS` | `worker.idle_interval_ms` | `OMNIOWN_WORKER_IDLE_INTERVAL_MS=30000` |

**注意：**

- 环境变量均为可选的，不存在时使用配置文件或内置默认值
- `OMNIOWN_ROOT` 会影响所有相对路径的解析
- `OMNIOWN_EMBEDDING_PROVIDER` 值不区分大小写：`mock` / `local`
