# Embedding 文档

OmniOwn 使用可插拔的 Provider 架构进行文本向量化，支持通过配置文件或 CLI 参数切换 embedding 实现。

---

## Provider 架构

核心 trait 定义在 `src/embedding.rs`：

```rust
pub trait EmbeddingProvider {
    fn model_name(&self) -> &str;
    fn dimension(&self) -> usize;
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
}
```

所有 embedding 操作通过 `Box<dyn EmbeddingProvider + Send + Sync>` 进行，支持工厂函数创建：

```rust
pub fn create_embedding_provider(
    kind: EmbeddingProviderKind,
    dim: usize,
) -> Result<Box<dyn EmbeddingProvider + Send + Sync>>
```

---

## 当前 Provider

### mock（默认）

| 属性 | 值 |
|------|-----|
| model_name | `mock-hash-{dim}`（如 `mock-hash-384`） |
| Status | available |
| 向量性质 | 确定性 hash 向量 |
| 是否语义模型 | ❌ 不是 |

`MockEmbeddingProvider` 基于文本的 SHA256 哈希生成固定维度的浮点向量：

- 相同文本 → 相同向量 ✅
- 不同文本 → 不同向量 ✅
- 输出向量经过 L2 归一化 ✅
- 空文本返回错误 ✅

用途：测试、开发、离线 fallback、验证 pipeline 完整性。

### local（stub）

| 属性 | 值 |
|------|-----|
| model_name | `local-stub` |
| Status | experimental / unavailable |
| 类型 | stub |

`LocalEmbeddingProvider` 当前是占位实现，调用 `embed()` 返回清晰错误：

```
LocalEmbeddingProvider is experimental and not enabled yet. Use --provider mock for now.
```

**不会 panic**，不会崩溃。

---

## model_name 隔离

`document_embeddings` 表通过 `(document_id, model_name)` 复合主键实现模型间完全隔离：

| document_id | model_name | 向量 |
|------------|------------|------|
| 1 | `mock-hash-384` | [...hash vectors...] |
| 1 | `local-stub` | [...stub...] |
| 1 | `nomic-embed-text` | （未来） |

**这意味着：**

- 切换 provider 时旧 embedding 不会被覆盖
- 同一文档可以同时拥有多个模型的 embedding
- `semantic-search --provider mock` 只搜索 `mock-hash-384` 的结果
- 不同模型的数据完全隔离，互不污染

---

## Pending 流程

> **pending 不再是文档级全局状态，而是模型级状态。**

### 示例

```
doc1 有 mock 的 embedding，但没有 local 的 embedding

对 mock provider：
  doc1 → 不是 pending ✅

对 local provider：
  doc1 → 是 pending ✅
```

### 查询逻辑

使用 `LEFT JOIN` 判断某个文档是否缺少特定模型的 embedding：

```sql
SELECT d.id, d.filename, d.content
FROM documents d
LEFT JOIN document_embeddings e
    ON d.id = e.document_id
   AND e.model_name = ?
WHERE e.document_id IS NULL          -- 没有该模型的 embedding
  AND d.processing_status = 'indexed'
  AND d.content IS NOT NULL
  AND TRIM(d.content) != ''
ORDER BY d.updated_at DESC
LIMIT ?;
```

### 对应的 Rust API

```rust
db::list_pending_embedding_documents(conn, model_name, limit)
db::count_pending_embeddings_for_model(conn, model_name)
db::count_embeddings_for_model(conn, model_name)
```

---

## Lazy Idle Embedding Worker

- 文档导入时**不立即**计算 embedding
- Worker 在系统空闲时小批量处理 pending 文档
- 空闲判定基于 `ActivityTracker`：无导入活动且距离上次活动超过阈值
- Worker 配置：

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `enabled` | `true` | 启用 |
| `idle_interval_ms` | `60000` | 轮询间隔（毫秒） |
| `batch_size` | `4` | 每批处理文档数 |

- 使用 `run_embedding_batch` 并传入 `provider.model_name()`
- Local stub 失败时不会导致 worker 崩溃，仅记录错误

---

## 向量工具

### 相似度计算

```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32
```

### 序列化

```rust
pub fn vector_to_blob(vector: &[f32]) -> Vec<u8>
pub fn blob_to_vector(blob: &[u8]) -> Result<Vec<f32>>
```

向量以二进制 BLOB 形式存储在 SQLite 中（每个 f32 4 字节，小端序）。
