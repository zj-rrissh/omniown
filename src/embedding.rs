use crate::db;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

// ---- Embedding Provider trait ----

pub trait EmbeddingProvider {
    fn model_name(&self) -> &str;
    fn dimension(&self) -> usize;
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

// ---- EmbeddingProviderKind ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProviderKind {
    #[default]
    Mock,
    Local,
}

impl<'de> Deserialize<'de> for EmbeddingProviderKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl EmbeddingProviderKind {
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mock" => Ok(Self::Mock),
            "local" => Ok(Self::Local),
            _ => Err(anyhow!(
                "unknown embedding provider '{}': valid options are mock, local",
                s
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Local => "local",
        }
    }
}

// ---- Mock Embedding Provider ----

#[derive(Debug, Clone)]
pub struct MockEmbeddingProvider {
    dim: usize,
    model_name: String,
}

impl MockEmbeddingProvider {
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            model_name: format!("mock-hash-{}", dim),
        }
    }
}

impl EmbeddingProvider for MockEmbeddingProvider {
    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let text = text.trim();
        if text.is_empty() {
            return Err(anyhow!("空文本无法生成 embedding"));
        }

        let mut vec = vec![0.0f32; self.dim];

        let hash = {
            let mut h = Sha256::new();
            h.update(text.as_bytes());
            h.finalize()
        };

        for chunk_idx in 0..(hash.len() / 4) {
            let start = chunk_idx * 4;
            let val = u32::from_le_bytes([
                hash[start],
                hash[start + 1],
                hash[start + 2],
                hash[start + 3],
            ]);
            let idx = (val as usize) % self.dim;
            let v = (val as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32;
            vec[idx] += v;
        }

        let bytes = text.as_bytes();
        for (bi, &byte) in bytes.iter().enumerate().take(self.dim * 2) {
            let idx = (bi * 7 + byte as usize * 13) % self.dim;
            let v = (byte as f32 / 255.0) * 0.1;
            vec[idx] += if bi % 2 == 0 { v } else { -v };
        }

        l2_normalize(&mut vec);
        Ok(vec)
    }
}

// ---- Local Embedding Provider ----

#[derive(Debug, Clone)]
pub struct LocalEmbeddingProvider {
    dim: usize,
    model_name: String,
}

impl LocalEmbeddingProvider {
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            model_name: local_model_name(dim),
        }
    }
}

impl EmbeddingProvider for LocalEmbeddingProvider {
    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[cfg(feature = "local-embedding")]
        {
            local_token_hash_embedding(text, self.dim)
        }
        #[cfg(not(feature = "local-embedding"))]
        {
            let _ = text;
            Err(anyhow!(
                "LocalEmbeddingProvider is experimental and not enabled yet. Build with --features local-embedding or use --provider mock."
            ))
        }
    }
}

#[cfg(feature = "local-embedding")]
fn local_model_name(dim: usize) -> String {
    format!("local-token-hash-{}", dim)
}

#[cfg(not(feature = "local-embedding"))]
fn local_model_name(_dim: usize) -> String {
    "local-stub".to_string()
}

#[cfg(feature = "local-embedding")]
fn local_token_hash_embedding(text: &str, dim: usize) -> Result<Vec<f32>> {
    if dim == 0 {
        return Err(anyhow!("embedding dimension must be greater than zero"));
    }

    let tokens = tokenize_for_local_embedding(text);
    if tokens.is_empty() {
        return Err(anyhow!("空文本无法生成 embedding"));
    }

    let mut vec = vec![0.0f32; dim];

    for token in &tokens {
        add_hashed_feature(&mut vec, token, 1.0);
    }

    for pair in tokens.windows(2) {
        let feature = format!("{} {}", pair[0], pair[1]);
        add_hashed_feature(&mut vec, &feature, 0.6);
    }

    l2_normalize(&mut vec);
    Ok(vec)
}

#[cfg(feature = "local-embedding")]
fn tokenize_for_local_embedding(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                current.push(lower);
            }
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

#[cfg(feature = "local-embedding")]
fn add_hashed_feature(vec: &mut [f32], feature: &str, weight: f32) {
    let hash = {
        let mut h = Sha256::new();
        h.update(feature.as_bytes());
        h.finalize()
    };

    for chunk_idx in 0..(hash.len() / 4) {
        let start = chunk_idx * 4;
        let val = u32::from_le_bytes([
            hash[start],
            hash[start + 1],
            hash[start + 2],
            hash[start + 3],
        ]);
        let idx = (val as usize) % vec.len();
        let sign = if val & 1 == 0 { 1.0 } else { -1.0 };
        vec[idx] += sign * weight;
    }
}

// ---- 工厂函数 ----

pub fn create_embedding_provider(
    kind: EmbeddingProviderKind,
    dim: usize,
) -> Result<Box<dyn EmbeddingProvider + Send + Sync>> {
    match kind {
        EmbeddingProviderKind::Mock => Ok(Box::new(MockEmbeddingProvider::new(dim))),
        EmbeddingProviderKind::Local => Ok(Box::new(LocalEmbeddingProvider::new(dim))),
    }
}

// ---- 向量运算 ----

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

fn l2_normalize(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in vec.iter_mut() {
            *v /= norm;
        }
    }
}

// ---- 向量序列化 ----

pub fn vector_to_blob(vector: &[f32]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(vector.len() * 4);
    for v in vector {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf
}

pub fn blob_to_vector(blob: &[u8]) -> Result<Vec<f32>> {
    if !blob.len().is_multiple_of(4) {
        return Err(anyhow!(
            "无效的向量 BLOB 长度: {} (not divisible by 4)",
            blob.len()
        ));
    }

    let mut vec = Vec::with_capacity(blob.len() / 4);
    for chunk in blob.chunks_exact(4) {
        vec.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(vec)
}

// ---- Pipeline ----

#[derive(Debug, Default)]
pub struct EmbeddingRunStats {
    pub done: usize,
    pub skipped: usize,
    pub failed: usize,
}

pub fn run_embedding_batch<P: EmbeddingProvider + ?Sized>(
    conn: &rusqlite::Connection,
    provider: &P,
    limit: usize,
) -> Result<EmbeddingRunStats> {
    let model_name = provider.model_name();
    let docs = db::list_pending_embedding_documents(conn, model_name, limit)?;
    let mut stats = EmbeddingRunStats::default();

    for doc in &docs {
        let content = match &doc.content {
            Some(c) if !c.trim().is_empty() => c.as_str(),
            _ => {
                db::update_embedding_status(conn, doc.id, "skipped")?;
                stats.skipped += 1;
                continue;
            }
        };

        match provider.embed(content) {
            Ok(vector) => {
                let dim = provider.dimension();
                let blob = vector_to_blob(&vector);
                db::upsert_document_embedding(conn, doc.id, provider.model_name(), dim, &blob)?;
                db::update_embedding_status(conn, doc.id, "done")?;
                stats.done += 1;
                println!("🧠 embedded: [{}] {}", doc.filename, doc.id);
            }
            Err(e) => {
                eprintln!("⚠️ embedding failed [{}]: {}", doc.filename, e);
                db::update_embedding_status(conn, doc.id, "failed")?;
                stats.failed += 1;
            }
        }
    }

    Ok(stats)
}

pub fn semantic_search<P: EmbeddingProvider + ?Sized>(
    conn: &rusqlite::Connection,
    provider: &P,
    query: &str,
    folder_type: Option<&str>,
    limit: usize,
) -> Result<Vec<db::SemanticSearchResult>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let query_vec = provider.embed(query)?;

    let rows =
        db::list_embeddings_for_search(conn, Some(provider.model_name()), folder_type, 10_000)?;

    let mut scored: Vec<db::SemanticSearchResult> = Vec::new();
    for r in rows {
        let vector = match blob_to_vector(&r.vector_blob) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("⚠️ 跳过损坏的向量 [{}]: {}", r.filename, e);
                continue;
            }
        };
        let score = cosine_similarity(&query_vec, &vector);
        scored.push(db::SemanticSearchResult {
            document_id: r.document_id,
            filename: r.filename,
            stored_path: r.stored_path,
            folder_type: r.folder_type,
            category: r.category,
            score,
        });
    }

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let limit = if limit == 0 { 10 } else { limit };
    scored.truncate(limit);

    Ok(scored)
}

// ---- 测试 ----

#[cfg(test)]
mod tests {
    use super::*;

    // ---- EmbeddingProviderKind ----

    #[test]
    fn provider_kind_parse_mock() {
        assert_eq!(
            EmbeddingProviderKind::parse("mock").unwrap(),
            EmbeddingProviderKind::Mock
        );
    }

    #[test]
    fn provider_kind_parse_local() {
        assert_eq!(
            EmbeddingProviderKind::parse("local").unwrap(),
            EmbeddingProviderKind::Local
        );
    }

    #[test]
    fn provider_kind_parse_case_insensitive() {
        assert_eq!(
            EmbeddingProviderKind::parse("MOCK").unwrap(),
            EmbeddingProviderKind::Mock
        );
        assert_eq!(
            EmbeddingProviderKind::parse("Local").unwrap(),
            EmbeddingProviderKind::Local
        );
        assert_eq!(
            EmbeddingProviderKind::parse(" LOCAL ").unwrap(),
            EmbeddingProviderKind::Local
        );
    }

    #[test]
    fn provider_kind_parse_invalid() {
        let err = EmbeddingProviderKind::parse("openai").unwrap_err();
        assert!(err.to_string().contains("unknown embedding provider"));
        assert!(err.to_string().contains("mock"));
        assert!(err.to_string().contains("local"));
    }

    #[test]
    fn provider_kind_as_str() {
        assert_eq!(EmbeddingProviderKind::Mock.as_str(), "mock");
        assert_eq!(EmbeddingProviderKind::Local.as_str(), "local");
    }

    #[test]
    fn provider_kind_default_is_mock() {
        assert_eq!(
            EmbeddingProviderKind::default(),
            EmbeddingProviderKind::Mock
        );
    }

    // ---- LocalEmbeddingProvider ----

    #[test]
    fn local_provider_constructs() {
        let p = LocalEmbeddingProvider::new(384);
        assert_eq!(p.dimension(), 384);
        #[cfg(feature = "local-embedding")]
        assert_eq!(p.model_name(), "local-token-hash-384");
        #[cfg(not(feature = "local-embedding"))]
        assert_eq!(p.model_name(), "local-stub");
    }

    #[test]
    #[cfg(not(feature = "local-embedding"))]
    fn local_provider_embed_returns_error() {
        let p = LocalEmbeddingProvider::new(384);
        let err = p.embed("test").unwrap_err();
        assert!(err.to_string().contains("experimental"));
    }

    #[test]
    #[cfg(feature = "local-embedding")]
    fn local_provider_embed_is_deterministic_and_normalized() {
        let p = LocalEmbeddingProvider::new(384);
        let v1 = p.embed("Rust async queue").unwrap();
        let v2 = p.embed("rust async queue").unwrap();
        assert_eq!(v1, v2);
        assert_eq!(v1.len(), 384);

        let norm: f32 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    // ---- create_embedding_provider ----

    #[test]
    fn create_mock_provider_works() {
        let p = create_embedding_provider(EmbeddingProviderKind::Mock, 384).unwrap();
        assert_eq!(p.model_name(), "mock-hash-384");
        assert_eq!(p.dimension(), 384);
        let v = p.embed("hello").unwrap();
        assert_eq!(v.len(), 384);
    }

    #[test]
    fn create_local_provider_constructs() {
        let p = create_embedding_provider(EmbeddingProviderKind::Local, 384).unwrap();
        assert_eq!(p.dimension(), 384);
        #[cfg(feature = "local-embedding")]
        assert!(p.embed("test").is_ok());
        #[cfg(not(feature = "local-embedding"))]
        assert!(p.embed("test").is_err());
    }

    // ---- MockEmbeddingProvider ----

    #[test]
    fn mock_embedding_same_text_same_vector() {
        let p = MockEmbeddingProvider::new(384);
        let v1 = p.embed("hello world").unwrap();
        let v2 = p.embed("hello world").unwrap();
        assert_eq!(v1.len(), 384);
        assert_eq!(v1, v2);
    }

    #[test]
    fn mock_embedding_different_text_different_vector() {
        let p = MockEmbeddingProvider::new(384);
        let v1 = p.embed("hello world").unwrap();
        let v2 = p.embed("goodbye world").unwrap();
        assert_ne!(v1, v2);
    }

    #[test]
    fn mock_embedding_dimension_is_fixed() {
        let p = MockEmbeddingProvider::new(384);
        assert_eq!(p.embed("test").unwrap().len(), 384);
        assert_eq!(p.dimension(), 384);
    }

    #[test]
    fn mock_embedding_output_is_normalized() {
        let p = MockEmbeddingProvider::new(384);
        let v = p.embed("some random text").unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_same_vector_approx_1() {
        let p = MockEmbeddingProvider::new(384);
        let v = p.embed("test").unwrap();
        let score = cosine_similarity(&v, &v);
        assert!((score - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_different_dimension_returns_0() {
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0, 0.0]), 0.0);
    }

    #[test]
    fn cosine_empty_returns_0() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn vector_blob_roundtrip() {
        let v = vec![0.1f32, -0.5, 0.75, 0.0, -0.25];
        let blob = vector_to_blob(&v);
        let v2 = blob_to_vector(&blob).unwrap();
        assert_eq!(v.len(), v2.len());
        for (a, b) in v.iter().zip(v2.iter()) {
            assert!((a - b).abs() < 1e-7);
        }
    }

    #[test]
    fn blob_to_vector_rejects_invalid_length() {
        let blob = vec![0u8; 7];
        assert!(blob_to_vector(&blob).is_err());
    }

    // ---- semantic_search model_name filtering ----

    #[test]
    fn semantic_search_filters_by_model_name() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();

        // Create tables manually (same as db tests pattern)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL,
                original_path TEXT,
                stored_path TEXT NOT NULL UNIQUE,
                file_ext TEXT,
                file_size INTEGER,
                file_hash TEXT NOT NULL,
                folder_type TEXT NOT NULL DEFAULT 'public',
                category TEXT NOT NULL DEFAULT 'misc',
                processing_status TEXT NOT NULL DEFAULT 'pending',
                embedding_status TEXT NOT NULL DEFAULT 'pending',
                content TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                imported_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS document_embeddings (
                document_id INTEGER NOT NULL,
                model_name TEXT NOT NULL,
                dim INTEGER NOT NULL,
                vector BLOB NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(document_id, model_name),
                FOREIGN KEY(document_id) REFERENCES documents(id) ON DELETE CASCADE
            );",
        )
        .unwrap();

        // Insert 3 documents directly via SQL
        conn.execute(
            "INSERT INTO documents (id, filename, stored_path, file_hash, folder_type, category, processing_status, embedding_status, content)
             VALUES (1, 'doc1.md', 'library/public/doc1.md', 'h1', 'public', 'notes', 'indexed', 'done', 'hello world one')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO documents (id, filename, stored_path, file_hash, folder_type, category, processing_status, embedding_status, content)
             VALUES (2, 'doc2.md', 'library/public/doc2.md', 'h2', 'public', 'notes', 'indexed', 'done', 'hello world two')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO documents (id, filename, stored_path, file_hash, folder_type, category, processing_status, embedding_status, content)
             VALUES (3, 'doc3.md', 'library/public/doc3.md', 'h3', 'public', 'notes', 'indexed', 'done', 'hello world three')",
            [],
        ).unwrap();

        // Create mock provider (model_name = "mock-hash-384")
        let provider = MockEmbeddingProvider::new(384);
        let model = provider.model_name().to_string();

        // Embed doc1 and doc2 with mock-hash-384
        let v1 = provider.embed("hello world one").unwrap();
        conn.execute(
            "INSERT INTO document_embeddings (document_id, model_name, dim, vector) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![1, model, 384, vector_to_blob(&v1)],
        ).unwrap();

        let v2 = provider.embed("hello world two").unwrap();
        conn.execute(
            "INSERT INTO document_embeddings (document_id, model_name, dim, vector) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![2, model, 384, vector_to_blob(&v2)],
        ).unwrap();

        // Embed doc3 with a DIFFERENT model name
        let v3 = provider.embed("hello world three").unwrap();
        conn.execute(
            "INSERT INTO document_embeddings (document_id, model_name, dim, vector) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![3, "other-model", 384, vector_to_blob(&v3)],
        ).unwrap();

        // Search with mock provider — should only return doc1 and doc2
        let results = semantic_search(&conn, &provider, "hello", None, 10).unwrap();
        assert_eq!(results.len(), 2);
        let ids: Vec<i64> = results.iter().map(|r| r.document_id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(!ids.contains(&3));
    }
}
