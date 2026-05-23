use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

/// OpenAI-compatible chat completion request (minimal subset)
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

/// Convert a natural-language query into effective FTS5 search terms using the LLM.
///
/// Examples:
///   "找一下上周写的那篇 Rust 异步笔记" → "rust async"
///   "关于数据库迁移的文档" → "database migration"
pub async fn generate_search_terms(
    query: &str,
    base_url: &str,
    model: &str,
    api_key: &str,
) -> Result<String> {
    let system_prompt = "You are a search query optimizer. Given a user's natural-language request, \
        extract 2-6 key search terms that would work well with full-text search (FTS5). \
        Return ONLY the search terms, one line per term group. \
        Do NOT include explanations, numbering, or formatting.\n\n\
        Examples:\n\
        User: 找一下关于 Rust 异步编程的文章\n\
        Terms: rust async\n\n\
        User: 数据库迁移方案那篇文档\n\
        Terms: database migration\n\n\
        User: 上周写的笔记关于配置环境变量\n\
        Terms: environment variable config setup\n\n\
        If the user's query is already concise (1-3 words), return it as-is.";

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: format!("User: {}\nTerms:", query),
            },
        ],
        temperature: 0.1,
        max_tokens: 100,
    };

    let client = reqwest::Client::new();
    let mut req = client
        .post(format!(
            "{}/chat/completions",
            base_url.trim_end_matches('/')
        ))
        .json(&request);

    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let resp = req
        .send()
        .await
        .with_context(|| format!("AI API 请求失败 (base_url={}, model={})", base_url, model))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "AI API 返回错误 ({}): {}",
            status,
            body.chars().take(200).collect::<String>()
        ));
    }

    let chat_resp: ChatResponse = resp.json().await.with_context(|| "解析 AI API 响应失败")?;

    let terms = chat_resp
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .unwrap_or_default();

    if terms.is_empty() {
        return Err(anyhow!("AI 返回了空的搜索词"));
    }

    Ok(terms)
}

/// Search documents using AI-generated terms, return results.
/// Intended for future MCP server use (kept for that purpose).
#[allow(dead_code)]
pub async fn ai_search(
    query: &str,
    base_url: &str,
    model: &str,
    api_key: &str,
    conn: &rusqlite::Connection,
) -> Result<Vec<crate::db::SearchResult>> {
    let terms = generate_search_terms(query, base_url, model, api_key).await?;

    let results =
        crate::db::search_documents(conn, &terms, 20).map_err(|e| anyhow!("搜索失败: {}", e))?;

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_serialization() {
        let req = ChatRequest {
            model: "gpt-4o-mini".to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "system prompt".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: "hello".to_string(),
                },
            ],
            temperature: 0.1,
            max_tokens: 100,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("gpt-4o-mini"));
        assert!(json.contains("system"));
        assert!(json.contains("user"));
    }

    #[test]
    fn chat_response_deserialization() {
        let json = r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "rust async"
                }
            }]
        }"#;

        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.content, "rust async");
    }

    #[test]
    fn chat_response_empty_choices() {
        let json = r#"{"choices": []}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices.is_empty());
    }
}
