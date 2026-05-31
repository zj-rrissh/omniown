# 数据库文档

OmniOwn 使用 SQLite 作为元数据和索引存储，通过 `rusqlite`（bundled 模式）驱动。

---

## schema_migrations

数据库版本迁移表：

```sql
CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

所有 schema 变更通过 migration 系统管理，而不是手动执行 DDL。

---

## documents

主文档表，存储每份文件的元数据和全文内容：

```sql
CREATE TABLE documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    filename TEXT NOT NULL,              -- 原始文件名
    original_path TEXT,                  -- 原始路径（inbox 中的路径）
    stored_path TEXT NOT NULL UNIQUE,    -- 存储路径（library/ 中的路径）
    file_ext TEXT,                       -- 文件扩展名
    file_size INTEGER,                   -- 文件大小（字节）
    file_hash TEXT NOT NULL,             -- SHA256 内容哈希（用于去重）
    folder_type TEXT NOT NULL DEFAULT 'public',   -- public / private
    category TEXT NOT NULL DEFAULT 'misc',         -- 分类（notes/code/finance/identity/journal）
    domain TEXT NOT NULL DEFAULT 'unknown',        -- 来源域
    doc_type TEXT NOT NULL DEFAULT 'unknown',      -- 文档类型
    content TEXT,                        -- 文件全文
    summary TEXT,                        -- 摘要（预留）
    tags TEXT,                           -- 标签（预留）
    privacy_score REAL DEFAULT 0,        -- 隐私分数（0-1）
    risk_level TEXT DEFAULT 'low',       -- 风险等级
    processing_status TEXT NOT NULL DEFAULT 'pending',  -- pending/indexed/failed
    embedding_status TEXT NOT NULL DEFAULT 'pending',   -- 文档级 embedding 状态
    summary_status TEXT NOT NULL DEFAULT 'skipped',     -- 摘要状态
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    imported_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

**关键字段说明：**

- `stored_path` — 文件在 `library/` 下的存储路径，也是业务唯一键
- `file_hash` — 提取正文的 SHA256 哈希，用于检测内容是否变更（`upsert` 时比较）
- `folder_type` — `public` / `private`，对应 `library/` 下的子目录
- `category` — 文档分类标签，由 `processor` 模块在导入时基于关键词自动分配
- `processing_status` — `pending` → `indexed` → 处理完成
- `embedding_status` — 文档级 embedding 状态。**已在 v0.1.0 废弃**，不再有代码写入或读取此字段。保留在 schema 中以保持向后兼容。

**索引：**

- `idx_documents_hash` (file_hash)
- `idx_documents_folder_type` (folder_type)
- `idx_documents_category` (category)
- `idx_documents_processing_status` (processing_status)
- `idx_documents_embedding_status` (embedding_status)
- `idx_documents_updated_at` (updated_at)

---

## documents_fts

FTS5 全文检索虚拟表：

```sql
CREATE VIRTUAL TABLE documents_fts USING fts5(
    filename,
    content,
    tags,
    summary,
    content='documents',
    content_rowid='id'
);
```

通过三个触发器与 `documents` 表保持同步：

- `documents_ai` — INSERT 时同步写入 FTS 索引
- `documents_ad` — DELETE 时从 FTS 索引删除
- `documents_au` — UPDATE 时删除旧索引并写入新索引

如需重建 FTS 索引，调用：

```rust
db::rebuild_fts_index(&conn)?;
```

**注意：** FTS5 可能受 SQLite 编译选项影响。如果当前 SQLite 未启用 FTS5，迁移会输出 WARN 但不会阻断其他功能。

---

## document_embeddings

> ⚠️ **已在 v0.1.0 废弃。** 不再有代码写入或读取此表。保留在 schema 中以保持向后兼容。

向量 embedding 存储表（历史参考）：

```sql
CREATE TABLE document_embeddings (
    document_id INTEGER NOT NULL,
    model_name TEXT NOT NULL,
    dim INTEGER NOT NULL,
    vector BLOB NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(document_id, model_name),
    FOREIGN KEY(document_id) REFERENCES documents(id) ON DELETE CASCADE
);
```

**复合主键设计：**

`PRIMARY KEY(document_id, model_name)` 允许同一文档保存多个模型的 embedding 向量：

| document_id | model_name | 说明 |
|------------|------------|------|
| 1 | `mock-hash-384` | Mock 确定性 hash |
| 1 | `local-token-hash-384` | Local provider（feature-gated 实验） |
| 1 | `nomic-embed-text` | 未来真实模型 |

**优点：**

- 切换 provider 不会覆盖旧 embedding
- 不同模型的 embedding 可以共存
- `semantic_search` 按 `model_name` 隔离，互不干扰

**索引：**

- `idx_document_embeddings_model_name` (model_name)
- `idx_document_embeddings_model_dim` (model_name, dim)

---

## Migrations 版本列表

| 版本 | 名称 | 说明 |
|------|------|------|
| 1 | `create_documents` | 创建 `documents` 表 |
| 2 | `create_documents_fts` | 创建 `documents_fts` 虚拟表与同步触发器 |
| 3 | `create_document_embeddings` | 创建 `document_embeddings` 表（旧单主键结构） |
| 4 | `create_indexes` | 创建常用索引 |
| 5 | `document_embeddings_composite_primary_key` | 升级为 `(document_id, model_name)` 复合主键 |

**迁移原则：**

- 所有迁移是幂等的（可重复执行）
- Migration 5 使用四步迁移（CREATE → COPY → DROP → RENAME）+ 事务保护
- 涉及表结构变更的迁移必须保证旧数据不丢失
