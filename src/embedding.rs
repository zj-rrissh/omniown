use crate::db;
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};

// ---- Embedding Provider trait ----

pub trait EmbeddingProvider {
    fn model_name(&self) -> &str;
    fn dimension(&self) -> usize;
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
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

pub fn run_embedding_batch<P: EmbeddingProvider>(
    conn: &rusqlite::Connection,
    provider: &P,
    limit: usize,
) -> Result<EmbeddingRunStats> {
    let docs = db::list_pending_embedding_documents(conn, limit)?;
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

pub fn semantic_search<P: EmbeddingProvider>(
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

    let rows = db::list_embeddings_for_search(conn, folder_type, 10_000)?;

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

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let limit = if limit == 0 { 10 } else { limit };
    scored.truncate(limit);

    Ok(scored)
}

// ---- 测试 ----

#[cfg(test)]
mod tests {
    use super::*;

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
}
