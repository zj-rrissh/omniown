use crate::config::AppConfig;
use crate::db;
use crate::fs_layout::AppPaths;
use rusqlite::Connection;
use serde_json::{Map, Value};
use std::io::{self, BufRead, Write};

// ---- MCP Server 入口 ----

pub fn run_mcp(config: &AppConfig, app_paths: &AppPaths) -> anyhow::Result<()> {
    let conn = Connection::open(&app_paths.db_path)?;

    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err = jsonrpc_error(None, PARSE_ERROR, "Parse error", &e.to_string());
                writeln!(stdout.lock(), "{}", err)?;
                continue;
            }
        };

        let response = handle_request(&request, &conn, config, app_paths);
        if let Some(resp) = response {
            writeln!(stdout.lock(), "{}", resp)?;
        }
        stdout.lock().flush()?;
    }

    Ok(())
}

// ---- JSON-RPC 常量 ----

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INTERNAL_ERROR: i64 = -32603;

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

// ---- 请求分发 ----

#[allow(unused_variables)]
fn handle_request(
    request: &Value,
    conn: &Connection,
    config: &AppConfig,
    paths: &AppPaths,
) -> Option<String> {
    let jsonrpc = request.get("jsonrpc").and_then(|v| v.as_str());
    if jsonrpc != Some("2.0") {
        return Some(jsonrpc_error(
            None,
            INVALID_REQUEST,
            "Invalid JSON-RPC version",
            "",
        ));
    }

    // 通知（无 id）→ 静默处理（notifications/initialized 等）
    let id = request.get("id")?;

    let method = match request.get("method").and_then(|v| v.as_str()) {
        Some(m) => m,
        None => {
            return Some(jsonrpc_error(
                Some(id),
                INVALID_REQUEST,
                "Missing method",
                "",
            ));
        }
    };

    let params = request.get("params").unwrap_or(&Value::Null);

    let result = match method {
        // MCP 协议方法
        "initialize" => handle_initialize(),
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tools_call(params, conn),

        // 兼容性/便利方法
        "ping" => Ok(Value::Null),

        _ => {
            return Some(jsonrpc_error(
                Some(id),
                METHOD_NOT_FOUND,
                &format!("Unknown method: {}", method),
                "",
            ));
        }
    };

    match result {
        Ok(result) => Some(jsonrpc_response(Some(id), result)),
        Err(e) => Some(jsonrpc_error(Some(id), INTERNAL_ERROR, &e.to_string(), "")),
    }
}

// ---- MCP: initialize ----

fn handle_initialize() -> anyhow::Result<Value> {
    Ok(serde_json::json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "omniown-mcp",
            "version": "0.1.0"
        }
    }))
}

// ---- MCP: tools/list ----

fn handle_tools_list() -> anyhow::Result<Value> {
    Ok(serde_json::json!({
        "tools": [
            {
                "name": "search_documents",
                "description": "Full-text search across all indexed documents using SQLite FTS5. Returns matching documents ranked by relevance.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search keywords (FTS5 query syntax supported)"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum results to return (default 20, max 100)"
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "get_document",
                "description": "Retrieve the full content and metadata of a single document by its database ID.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "number",
                            "description": "Document ID (integer)"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "list_documents",
                "description": "List recently updated documents with metadata (no content). Optionally filter by folder type (public/private).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "folder_type": {
                            "type": "string",
                            "description": "Filter by folder: \"public\" or \"private\""
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum documents to return (default 50, max 200)"
                        }
                    },
                    "required": []
                }
            },
            {
                "name": "get_status",
                "description": "Get knowledge base statistics: document counts, schema version, and index health.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        ]
    }))
}

// ---- MCP: tools/call ----

fn handle_tools_call(params: &Value, conn: &Connection) -> anyhow::Result<Value> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;

    let arguments = params.get("arguments").unwrap_or(&Value::Null);

    let result = match name {
        "search_documents" => tool_search_documents(arguments, conn)?,
        "get_document" => tool_get_document(arguments, conn)?,
        "list_documents" => tool_list_documents(arguments, conn)?,
        "get_status" => tool_get_status(conn)?,
        _ => anyhow::bail!("Unknown tool: {}", name),
    };

    Ok(result)
}

// ---- 工具实现 ----

fn tool_search_documents(params: &Value, conn: &Connection) -> anyhow::Result<Value> {
    let query = params
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: query"))?
        .trim();

    if query.is_empty() {
        return Ok(serde_json::json!({
            "content": [{"type": "text", "text": "[]"}],
            "isError": false
        }));
    }

    let limit = params
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(20)
        .clamp(1, 100);

    let results = db::search_documents(conn, query, limit)?;

    let items: Vec<Value> = results
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "filename": r.filename,
                "stored_path": r.stored_path,
                "folder_type": r.folder_type,
                "category": r.category,
                "snippet": r.snippet,
                "rank": r.rank,
                "updated_at": r.updated_at
            })
        })
        .collect();

    let json_str = serde_json::to_string(&items)?;

    Ok(serde_json::json!({
        "content": [{"type": "text", "text": json_str}],
        "isError": false
    }))
}

fn tool_get_document(params: &Value, conn: &Connection) -> anyhow::Result<Value> {
    let id = params.get("id").and_then(|v| v.as_i64()).ok_or_else(|| {
        anyhow::anyhow!("Missing or invalid required parameter: id (must be integer)")
    })?;

    match db::get_document_by_id(conn, id)? {
        Some(doc) => {
            let json_str = serde_json::to_string(&serde_json::json!({
                "id": doc.id,
                "filename": doc.filename,
                "original_path": doc.original_path,
                "stored_path": doc.stored_path,
                "file_ext": doc.file_ext,
                "file_size": doc.file_size,
                "file_hash": doc.file_hash,
                "folder_type": doc.folder_type,
                "category": doc.category,
                "domain": doc.domain,
                "doc_type": doc.doc_type,
                "content": doc.content,
                "summary": doc.summary,
                "tags": doc.tags,
                "privacy_score": doc.privacy_score,
                "risk_level": doc.risk_level,
                "processing_status": doc.processing_status,
                "summary_status": doc.summary_status,
                "created_at": doc.created_at,
                "updated_at": doc.updated_at,
                "imported_at": doc.imported_at
            }))?;

            Ok(serde_json::json!({
                "content": [{"type": "text", "text": json_str}],
                "isError": false
            }))
        }
        None => {
            let msg = format!("Document with id={} not found", id);
            Ok(serde_json::json!({
                "content": [{"type": "text", "text": msg}],
                "isError": true
            }))
        }
    }
}

fn tool_list_documents(params: &Value, conn: &Connection) -> anyhow::Result<Value> {
    let limit = params
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(50)
        .clamp(1, 200);

    let docs = if let Some(folder) = params.get("folder_type").and_then(|v| v.as_str()) {
        if folder != "public" && folder != "private" {
            anyhow::bail!(
                "Invalid folder_type: '{}' (must be 'public' or 'private')",
                folder
            );
        }
        db::list_by_folder_type(conn, folder)?
    } else {
        db::list_documents_meta_limited(conn, limit)?
    };

    let items: Vec<Value> = docs
        .into_iter()
        .map(|d| {
            serde_json::json!({
                "id": d.id,
                "filename": d.filename,
                "stored_path": d.stored_path,
                "file_ext": d.file_ext,
                "file_size": d.file_size,
                "folder_type": d.folder_type,
                "category": d.category,
                "risk_level": d.risk_level,
                "updated_at": d.updated_at
            })
        })
        .collect();

    let json_str = serde_json::to_string(&items)?;

    Ok(serde_json::json!({
        "content": [{"type": "text", "text": json_str}],
        "isError": false
    }))
}

fn tool_get_status(conn: &Connection) -> anyhow::Result<Value> {
    let total = db::count_documents(conn).unwrap_or(0);
    let public = db::count_by_folder_type(conn, "public").unwrap_or(0);
    let private = db::count_by_folder_type(conn, "private").unwrap_or(0);
    let indexed = db::count_by_processing_status(conn, "indexed").unwrap_or(0);
    let failed = db::count_by_processing_status(conn, "failed").unwrap_or(0);
    let schema_version = crate::migration::current_version(conn).unwrap_or(0);

    let json_str = serde_json::to_string(&serde_json::json!({
        "documents": {
            "total": total,
            "public": public,
            "private": private,
            "indexed": indexed,
            "failed": failed
        },
        "schema_version": schema_version
    }))?;

    Ok(serde_json::json!({
        "content": [{"type": "text", "text": json_str}],
        "isError": false
    }))
}

// ---- JSON-RPC 辅助函数 ----

fn jsonrpc_response(id: Option<&Value>, result: Value) -> String {
    let mut resp = Map::new();
    resp.insert("jsonrpc".to_string(), Value::String("2.0".to_string()));
    if let Some(id_val) = id {
        resp.insert("id".to_string(), id_val.clone());
    }
    resp.insert("result".to_string(), result);
    serde_json::to_string(&Value::Object(resp)).unwrap_or_default()
}

fn jsonrpc_error(id: Option<&Value>, code: i64, message: &str, data: &str) -> String {
    let mut err = Map::new();
    err.insert(
        "code".to_string(),
        Value::Number(serde_json::Number::from(code)),
    );
    err.insert("message".to_string(), Value::String(message.to_string()));
    if !data.is_empty() {
        err.insert("data".to_string(), Value::String(data.to_string()));
    }

    let mut resp = Map::new();
    resp.insert("jsonrpc".to_string(), Value::String("2.0".to_string()));
    if let Some(id_val) = id {
        resp.insert("id".to_string(), id_val.clone());
    } else {
        resp.insert("id".to_string(), Value::Null);
    }
    resp.insert("error".to_string(), Value::Object(err));
    serde_json::to_string(&Value::Object(resp)).unwrap_or_default()
}

// ---- 测试 ----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rusqlite::Connection;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static MCP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn setup_db() -> (Connection, AppPaths, AppConfig) {
        let counter = MCP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "omniown_mcp_test_{}_{}",
            std::process::id(),
            counter
        ));
        std::fs::create_dir_all(&root).unwrap();
        let paths = crate::fs_layout::AppPaths::new(&root);
        paths.init_directories().unwrap();
        db::init_database(&paths.db_path).unwrap();
        let conn = Connection::open(&paths.db_path).unwrap();
        let config = AppConfig::default();
        (conn, paths, config)
    }

    fn insert_doc(
        conn: &Connection,
        filename: &str,
        stored_path: &str,
        content: &str,
        folder_type: &str,
    ) {
        let input = db::NewDocument {
            filename,
            original_path: None,
            stored_path,
            content,
            folder_type,
            category: "notes",
            domain: "general",
            doc_type: "text",
            file_ext: Some("txt"),
            file_size: Some(content.len() as i64),
            summary: None,
            tags: None,
            privacy_score: 0.0,
            risk_level: "low",
            processing_status: "indexed",
            summary_status: "skipped",
        };
        db::upsert_document(conn, &input).unwrap();
    }

    // ---- 请求/响应解析测试 ----

    #[test]
    fn test_initialize_handshake() {
        let (conn, paths, config) = setup_db();
        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#
        ).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();

        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 1);
        assert!(resp["result"]["protocolVersion"].is_string());
        assert_eq!(resp["result"]["serverInfo"]["name"], "omniown-mcp");
    }

    #[test]
    fn test_tools_list() {
        let (conn, paths, config) = setup_db();
        let req: Value =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#)
                .unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();

        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 2);

        let tools = resp["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"search_documents"));
        assert!(names.contains(&"get_document"));
        assert!(names.contains(&"list_documents"));
        assert!(names.contains(&"get_status"));
    }

    #[test]
    fn test_notification_is_ignored() {
        let (conn, paths, config) = setup_db();
        let req: Value =
            serde_json::from_str(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .unwrap();

        let resp = handle_request(&req, &conn, &config, &paths);
        assert!(
            resp.is_none(),
            "notifications should not produce a response"
        );
    }

    #[test]
    fn test_ping() {
        let (conn, paths, config) = setup_db();
        let req: Value =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":3,"method":"ping","params":{}}"#)
                .unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["result"], Value::Null);
    }

    #[test]
    fn test_unknown_method() {
        let (conn, paths, config) = setup_db();
        let req: Value =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":99,"method":"unknown","params":{}}"#)
                .unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["error"]["code"], METHOD_NOT_FOUND);
    }

    // ---- 工具测试 ----

    #[test]
    fn test_search_documents() {
        let (conn, paths, config) = setup_db();
        insert_doc(
            &conn,
            "hello.txt",
            "library/public/hello.txt",
            "Hello world content",
            "public",
        );
        insert_doc(
            &conn,
            "other.txt",
            "library/public/other.txt",
            "Something else",
            "public",
        );

        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"search_documents","arguments":{"query":"Hello"}}}"#
        ).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["id"], 10);

        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("hello.txt"), "search should find hello.txt");
        assert!(
            !text.contains("other.txt"),
            "search should not find other.txt"
        );
    }

    #[test]
    fn test_search_documents_empty_query() {
        let (conn, paths, config) = setup_db();
        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"search_documents","arguments":{"query":""}}}"#
        ).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert_eq!(text, "[]");
    }

    #[test]
    fn test_get_document() {
        let (conn, paths, config) = setup_db();
        insert_doc(
            &conn,
            "doc.txt",
            "library/public/doc.txt",
            "Document content here",
            "public",
        );

        // 先搜索获取 ID
        let results = db::search_documents(&conn, "Document", 10).unwrap();
        assert!(!results.is_empty());
        let doc_id = results[0].id;

        let req_json = format!(
            r#"{{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{{"name":"get_document","arguments":{{"id":{}}}}}}}"#,
            doc_id
        );
        let req: Value = serde_json::from_str(&req_json).unwrap();
        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();

        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Document content here"));
    }

    #[test]
    fn test_get_document_not_found() {
        let (conn, paths, config) = setup_db();
        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"get_document","arguments":{"id":99999}}}"#
        ).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn test_list_documents() {
        let (conn, paths, config) = setup_db();
        insert_doc(&conn, "a.txt", "library/public/a.txt", "AAA", "public");
        insert_doc(&conn, "b.txt", "library/private/b.txt", "BBB", "private");

        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":30,"method":"tools/call","params":{"name":"list_documents","arguments":{}}}"#
        ).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("a.txt"));
        assert!(text.contains("b.txt"));
    }

    #[test]
    fn test_list_documents_filtered() {
        let (conn, paths, config) = setup_db();
        insert_doc(&conn, "pub.txt", "library/public/pub.txt", "PUB", "public");
        insert_doc(
            &conn,
            "priv.txt",
            "library/private/priv.txt",
            "PRIV",
            "private",
        );

        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":31,"method":"tools/call","params":{"name":"list_documents","arguments":{"folder_type":"public"}}}"#
        ).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("pub.txt"));
        assert!(
            !text.contains("priv.txt"),
            "private docs should be filtered out"
        );
    }

    #[test]
    fn test_list_documents_invalid_folder() {
        let (conn, paths, config) = setup_db();
        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":32,"method":"tools/call","params":{"name":"list_documents","arguments":{"folder_type":"invalid"}}}"#
        ).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        // Should return error for invalid folder type
        assert_eq!(resp["error"]["code"], INTERNAL_ERROR);
        assert!(
            resp["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Invalid folder_type")
        );
    }

    #[test]
    fn test_get_status() {
        let (conn, paths, config) = setup_db();
        insert_doc(&conn, "s.txt", "library/public/s.txt", "STATUS", "public");

        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"get_status","arguments":{}}}"#
        ).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            text.contains("\"total\":1"),
            "status should show total=1, got: {}",
            text
        );
        assert!(text.contains("\"public\":1"), "status should show public=1");
    }

    // ---- JSON-RPC 协议层测试 ----

    #[test]
    fn test_invalid_json() {
        let (conn, paths, config) = setup_db();
        // 不是 JSON 输入 — 通过 handle_request 模拟 parse error
        // 实际上 parse error 在 run_mcp 中处理，这里测试 handle_request 直接
        // 测试 JSON-RPC 版本检查
        let req: Value =
            serde_json::from_str(r#"{"id":1,"method":"initialize","params":{}}"#).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["error"]["code"], INVALID_REQUEST);
    }

    #[test]
    fn test_missing_method() {
        let (conn, paths, config) = setup_db();
        let req: Value = serde_json::from_str(r#"{"jsonrpc":"2.0","id":1,"params":{}}"#).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["error"]["code"], INVALID_REQUEST);
    }

    #[test]
    fn test_tool_missing_name() {
        let (conn, paths, config) = setup_db();
        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"arguments":{}}}"#,
        )
        .unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["error"]["code"], INTERNAL_ERROR);
    }

    #[test]
    fn test_search_missing_query() {
        let (conn, paths, config) = setup_db();
        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":51,"method":"tools/call","params":{"name":"search_documents","arguments":{}}}"#
        ).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["error"]["code"], INTERNAL_ERROR);
    }

    #[test]
    fn test_get_document_missing_id() {
        let (conn, paths, config) = setup_db();
        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":52,"method":"tools/call","params":{"name":"get_document","arguments":{}}}"#
        ).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["error"]["code"], INTERNAL_ERROR);
    }

    #[test]
    fn test_unknown_tool() {
        let (conn, paths, config) = setup_db();
        let req: Value = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":60,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}"#
        ).unwrap();

        let resp_str = handle_request(&req, &conn, &config, &paths).unwrap();
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["error"]["code"], INTERNAL_ERROR);
    }

    // ---- JSON-RPC 辅助函数测试 ----

    #[test]
    fn test_jsonrpc_error_without_data() {
        let err = jsonrpc_error(None, -32601, "Method not found", "");
        let v: Value = serde_json::from_str(&err).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["error"]["code"], -32601);
    }

    #[test]
    fn test_jsonrpc_error_with_id() {
        let id = Value::Number(serde_json::Number::from(42));
        let err = jsonrpc_error(Some(&id), -32602, "Bad params", "missing field");
        let v: Value = serde_json::from_str(&err).unwrap();
        assert_eq!(v["id"], 42);
        assert_eq!(v["error"]["data"], "missing field");
    }
}
