use crate::config::AppConfig;
use crate::db::{self, Document, SearchResult};
use crate::embedding;
use crate::fs_layout::AppPaths;
use crate::migration;
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 17777,
        }
    }
}

pub fn run_server(
    config: &AppConfig,
    app_paths: &AppPaths,
    serve: ServeConfig,
) -> anyhow::Result<()> {
    app_paths.init_directories()?;
    db::init_database(&app_paths.db_path)?;

    let addr = format!("{}:{}", serve.host, serve.port);
    let listener = TcpListener::bind(&addr)?;
    println!("OmniOwn UI: http://{addr}");
    println!("Press Ctrl+C to stop.\n");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(err) = handle_stream(&mut stream, config, app_paths) {
                    eprintln!("HTTP request failed: {err:#}");
                }
            }
            Err(err) => eprintln!("HTTP accept failed: {err}"),
        }
    }

    Ok(())
}

fn handle_stream(
    stream: &mut TcpStream,
    config: &AppConfig,
    app_paths: &AppPaths,
) -> anyhow::Result<()> {
    let mut buffer = [0_u8; 8192];
    let read = stream.read(&mut buffer)?;
    if read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..read]);
    let response = handle_request(&request, config, app_paths);
    stream.write_all(&response.to_bytes())?;
    stream.flush()?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpResponse {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: String,
}

impl HttpResponse {
    fn html(body: String) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type: "text/html; charset=utf-8",
            body,
        }
    }

    fn json(body: String) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type: "application/json; charset=utf-8",
            body,
        }
    }

    fn text(body: String, content_type: &'static str) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type,
            body,
        }
    }

    fn error(status: u16, reason: &'static str, message: &str) -> Self {
        Self {
            status,
            reason,
            content_type: "application/json; charset=utf-8",
            body: format!(
                "{{\"error\":{{\"status\":{},\"message\":{}}}}}",
                status,
                json_string(message)
            ),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let body = self.body.as_bytes();
        let head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
            self.status,
            self.reason,
            self.content_type,
            body.len()
        );
        [head.as_bytes(), body].concat()
    }
}

fn handle_request(request: &str, config: &AppConfig, app_paths: &AppPaths) -> HttpResponse {
    let Some(first_line) = request.lines().next() else {
        return HttpResponse::error(400, "Bad Request", "empty request");
    };

    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");

    if method != "GET" && method != "HEAD" {
        return HttpResponse::error(405, "Method Not Allowed", "only GET is supported");
    }

    let (path, query) = split_target(target);

    let mut response = match path.as_str() {
        "/" | "/index.html" => HttpResponse::html(index_html(app_paths)),
        "/api/status" => api_status(config, app_paths),
        "/api/documents" => api_documents(app_paths, &query),
        "/api/search" => api_search(app_paths, &query),
        _ if path.starts_with("/api/documents/") => {
            let id = path.trim_start_matches("/api/documents/");
            api_document_detail(app_paths, id)
        }
        _ => static_file_response(app_paths, &path)
            .unwrap_or_else(|| HttpResponse::error(404, "Not Found", "not found")),
    };

    if method == "HEAD" {
        response.body.clear();
    }

    response
}

fn api_status(config: &AppConfig, app_paths: &AppPaths) -> HttpResponse {
    let conn = match Connection::open(&app_paths.db_path) {
        Ok(conn) => conn,
        Err(err) => return HttpResponse::error(500, "Internal Server Error", &err.to_string()),
    };

    let total = db::count_documents(&conn).unwrap_or(0);
    let public = db::count_by_folder_type(&conn, "public").unwrap_or(0);
    let private = db::count_by_folder_type(&conn, "private").unwrap_or(0);
    let indexed = db::count_by_processing_status(&conn, "indexed").unwrap_or(0);
    let failed = db::count_by_processing_status(&conn, "failed").unwrap_or(0);
    let embeddings = db::count_embeddings(&conn).unwrap_or(0);
    let pending_embeddings = db::count_pending_embeddings(&conn).unwrap_or(0);
    let schema_version = migration::current_version(&conn).unwrap_or(0);
    let pending_migrations = migration::pending_count(&conn).unwrap_or(-1);

    let (model_name, model_embeddings, model_pending) =
        match embedding::create_embedding_provider(config.embedding.provider, config.embedding.dim)
        {
            Ok(provider) => {
                let model_name = provider.model_name().to_string();
                let model_embeddings =
                    db::count_embeddings_for_model(&conn, &model_name).unwrap_or(0);
                let model_pending =
                    db::count_pending_embeddings_for_model(&conn, &model_name).unwrap_or(0);
                (model_name, model_embeddings, model_pending)
            }
            Err(_) => ("unknown".to_string(), 0, pending_embeddings),
        };

    HttpResponse::json(format!(
        "{{\"database\":{},\"root\":{},\"schema\":{{\"current_version\":{},\"pending_migrations\":{}}},\"documents\":{{\"total\":{},\"public\":{},\"private\":{},\"indexed\":{},\"failed\":{}}},\"embeddings\":{{\"total\":{},\"pending\":{},\"current_model\":{},\"current_model_embeddings\":{},\"pending_for_current_model\":{}}},\"worker\":{{\"enabled\":{}}}}}",
        json_string(&app_paths.db_path.display().to_string()),
        json_string(&app_paths.root.display().to_string()),
        schema_version,
        pending_migrations,
        total,
        public,
        private,
        indexed,
        failed,
        embeddings,
        pending_embeddings,
        json_string(&model_name),
        model_embeddings,
        model_pending,
        config.worker.enabled
    ))
}

fn api_documents(app_paths: &AppPaths, query: &HashMap<String, String>) -> HttpResponse {
    let conn = match Connection::open(&app_paths.db_path) {
        Ok(conn) => conn,
        Err(err) => return HttpResponse::error(500, "Internal Server Error", &err.to_string()),
    };
    let limit = query_limit(query, DEFAULT_LIMIT);

    match db::list_documents_meta_limited(&conn, limit) {
        Ok(docs) => {
            let body = format!(
                "{{\"limit\":{},\"documents\":[{}]}}",
                limit,
                docs.iter()
                    .map(document_summary_json)
                    .collect::<Vec<_>>()
                    .join(",")
            );
            HttpResponse::json(body)
        }
        Err(err) => HttpResponse::error(500, "Internal Server Error", &err.to_string()),
    }
}

fn api_search(app_paths: &AppPaths, query: &HashMap<String, String>) -> HttpResponse {
    let conn = match Connection::open(&app_paths.db_path) {
        Ok(conn) => conn,
        Err(err) => return HttpResponse::error(500, "Internal Server Error", &err.to_string()),
    };
    let limit = query_limit(query, 20);
    let q = query.get("q").map(String::as_str).unwrap_or("").trim();

    match db::search_documents(&conn, q, limit) {
        Ok(results) => {
            let body = format!(
                "{{\"query\":{},\"limit\":{},\"results\":[{}]}}",
                json_string(q),
                limit,
                results
                    .iter()
                    .map(search_result_json)
                    .collect::<Vec<_>>()
                    .join(",")
            );
            HttpResponse::json(body)
        }
        Err(err) => HttpResponse::error(500, "Internal Server Error", &err.to_string()),
    }
}

fn api_document_detail(app_paths: &AppPaths, id: &str) -> HttpResponse {
    let Ok(id) = id.parse::<i64>() else {
        return HttpResponse::error(400, "Bad Request", "document id must be an integer");
    };

    let conn = match Connection::open(&app_paths.db_path) {
        Ok(conn) => conn,
        Err(err) => return HttpResponse::error(500, "Internal Server Error", &err.to_string()),
    };

    match db::get_document_by_id(&conn, id) {
        Ok(Some(doc)) => {
            HttpResponse::json(format!("{{\"document\":{}}}", document_detail_json(&doc)))
        }
        Ok(None) => HttpResponse::error(404, "Not Found", "document not found"),
        Err(err) => HttpResponse::error(500, "Internal Server Error", &err.to_string()),
    }
}

fn query_limit(query: &HashMap<String, String>, default: i64) -> i64 {
    query
        .get("limit")
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|limit| *limit > 0)
        .map(|limit| limit.min(MAX_LIMIT))
        .unwrap_or(default)
}

fn document_summary_json(doc: &Document) -> String {
    format!(
        "{{\"id\":{},\"filename\":{},\"stored_path\":{},\"folder_type\":{},\"category\":{},\"risk_level\":{},\"processing_status\":{},\"embedding_status\":{},\"updated_at\":{},\"file_ext\":{},\"file_size\":{}}}",
        doc.id,
        json_string(&doc.filename),
        json_string(&doc.stored_path),
        json_string(&doc.folder_type),
        json_string(&doc.category),
        json_string(&doc.risk_level),
        json_string(&doc.processing_status),
        json_string(&doc.embedding_status),
        json_string(&doc.updated_at),
        json_option(doc.file_ext.as_deref()),
        json_i64_option(doc.file_size)
    )
}

fn document_detail_json(doc: &Document) -> String {
    format!(
        "{{\"id\":{},\"filename\":{},\"original_path\":{},\"stored_path\":{},\"file_ext\":{},\"file_size\":{},\"folder_type\":{},\"category\":{},\"domain\":{},\"doc_type\":{},\"content\":{},\"summary\":{},\"tags\":{},\"privacy_score\":{},\"risk_level\":{},\"processing_status\":{},\"embedding_status\":{},\"summary_status\":{},\"created_at\":{},\"updated_at\":{},\"imported_at\":{}}}",
        doc.id,
        json_string(&doc.filename),
        json_option(doc.original_path.as_deref()),
        json_string(&doc.stored_path),
        json_option(doc.file_ext.as_deref()),
        json_i64_option(doc.file_size),
        json_string(&doc.folder_type),
        json_string(&doc.category),
        json_string(&doc.domain),
        json_string(&doc.doc_type),
        json_option(doc.content.as_deref()),
        json_option(doc.summary.as_deref()),
        json_option(doc.tags.as_deref()),
        doc.privacy_score,
        json_string(&doc.risk_level),
        json_string(&doc.processing_status),
        json_string(&doc.embedding_status),
        json_string(&doc.summary_status),
        json_string(&doc.created_at),
        json_string(&doc.updated_at),
        json_string(&doc.imported_at)
    )
}

fn search_result_json(result: &SearchResult) -> String {
    format!(
        "{{\"id\":{},\"filename\":{},\"stored_path\":{},\"folder_type\":{},\"category\":{},\"snippet\":{},\"rank\":{},\"updated_at\":{}}}",
        result.id,
        json_string(&result.filename),
        json_string(&result.stored_path),
        json_string(&result.folder_type),
        json_string(&result.category),
        json_option(result.snippet.as_deref()),
        result.rank,
        json_string(&result.updated_at)
    )
}

fn json_option(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_string())
}

fn json_i64_option(value: Option<i64>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch < '\u{20}' => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn split_target(target: &str) -> (String, HashMap<String, String>) {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut query_map = HashMap::new();

    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        query_map.insert(percent_decode(key), percent_decode(value));
    }

    (percent_decode(path), query_map)
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3])
                    && let Ok(byte) = u8::from_str_radix(hex, 16)
                {
                    out.push(byte);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).to_string()
}

fn index_html(app_paths: &AppPaths) -> String {
    let index = frontend_dist_dir(app_paths).join("index.html");
    fs::read_to_string(index).unwrap_or_else(|_| missing_ui_html())
}

fn static_file_response(app_paths: &AppPaths, request_path: &str) -> Option<HttpResponse> {
    let relative = request_path.strip_prefix('/')?;
    if relative.is_empty() || !is_safe_static_path(relative) {
        return None;
    }

    let file_path = frontend_dist_dir(app_paths).join(relative);
    if !file_path.is_file() {
        return None;
    }

    let body = fs::read_to_string(&file_path).ok()?;
    Some(HttpResponse::text(body, content_type(&file_path)))
}

fn is_safe_static_path(relative: &str) -> bool {
    Path::new(relative)
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "css" => "text/css; charset=utf-8",
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        _ => "text/plain; charset=utf-8",
    }
}

fn frontend_dist_dir(app_paths: &AppPaths) -> PathBuf {
    let configured = app_paths.root.join("ui").join("dist");
    if configured.join("index.html").is_file() {
        return configured;
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("ui")
        .join("dist")
}

fn missing_ui_html() -> String {
    r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>OmniOwn UI</title>
  <style>
    body { margin: 0; font-family: system-ui, sans-serif; background: #f7f8f9; color: #202124; }
    main { max-width: 720px; margin: 12vh auto; padding: 0 24px; }
    h1 { font-size: 28px; margin: 0 0 12px; }
    p { color: #626970; line-height: 1.6; }
    code { background: #fff; border: 1px solid #d9dde2; border-radius: 6px; padding: 2px 6px; }
  </style>
</head>
<body>
  <main>
    <h1>OmniOwn UI build is missing</h1>
    <p>The Rust API is running, but the Vue + TypeScript frontend has not been built into <code>ui/dist</code>.</p>
    <p>Build it with <code>cd ui && npm install && npm run build</code>, then restart <code>cargo run -- serve</code>.</p>
  </main>
</body>
</html>"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NewDocument;
    use std::fs;
    use std::path::PathBuf;

    fn make_temp_app(name: &str) -> (AppConfig, AppPaths, PathBuf) {
        let root = std::env::temp_dir().join(format!("omniown_ui_{}_{}", name, std::process::id()));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(&root).unwrap();
        let paths = AppPaths::new(&root);
        paths.init_directories().unwrap();
        db::init_database(&paths.db_path).unwrap();
        (AppConfig::default(), paths, root)
    }

    fn insert_doc(paths: &AppPaths, filename: &str, stored_path: &str, content: &str) -> i64 {
        let conn = Connection::open(&paths.db_path).unwrap();
        let input = NewDocument {
            filename,
            original_path: None,
            stored_path,
            content,
            folder_type: "public",
            category: "notes",
            domain: "unknown",
            doc_type: "markdown",
            file_ext: Some("md"),
            file_size: Some(content.len() as i64),
            summary: None,
            tags: None,
            privacy_score: 0.1,
            risk_level: "low",
            processing_status: "indexed",
            embedding_status: "pending",
            summary_status: "skipped",
        };
        db::upsert_document(&conn, &input).unwrap().1.id
    }

    #[test]
    fn status_api_reports_empty_database() {
        let (config, paths, root) = make_temp_app("status");
        let response = handle_request("GET /api/status HTTP/1.1\r\n\r\n", &config, &paths);

        assert_eq!(response.status, 200);
        assert!(response.body.contains("\"total\":0"));
        assert!(response.body.contains("\"current_version\":5"));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn documents_api_limits_results_and_omits_content() {
        let (config, paths, root) = make_temp_app("documents");
        insert_doc(&paths, "one.md", "library/public/one.md", "one hidden body");
        insert_doc(&paths, "two.md", "library/public/two.md", "two hidden body");

        let response = handle_request(
            "GET /api/documents?limit=1 HTTP/1.1\r\n\r\n",
            &config,
            &paths,
        );

        assert_eq!(response.status, 200);
        assert_eq!(response.body.matches("\"filename\"").count(), 1);
        assert!(!response.body.contains("hidden body"));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn search_api_uses_fts_and_empty_query_returns_empty_results() {
        let (config, paths, root) = make_temp_app("search");
        insert_doc(
            &paths,
            "alpha.md",
            "library/public/alpha.md",
            "AlphaBeta searchable content",
        );

        let response = handle_request(
            "GET /api/search?q=AlphaBeta HTTP/1.1\r\n\r\n",
            &config,
            &paths,
        );
        assert_eq!(response.status, 200);
        assert!(response.body.contains("alpha.md"));

        let empty = handle_request("GET /api/search?q= HTTP/1.1\r\n\r\n", &config, &paths);
        assert_eq!(empty.status, 200);
        assert!(empty.body.contains("\"results\":[]"));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn document_detail_missing_id_returns_404() {
        let (config, paths, root) = make_temp_app("missing");
        let response = handle_request("GET /api/documents/404 HTTP/1.1\r\n\r\n", &config, &paths);

        assert_eq!(response.status, 404);
        assert!(response.body.contains("document not found"));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn serves_vue_dist_static_assets() {
        let (config, paths, root) = make_temp_app("static");
        let dist = root.join("ui").join("dist");
        fs::create_dir_all(dist.join("assets")).unwrap();
        fs::write(dist.join("index.html"), "<div id=\"app\"></div>").unwrap();
        fs::write(dist.join("assets").join("app.js"), "console.log('vue')").unwrap();

        let index = handle_request("GET / HTTP/1.1\r\n\r\n", &config, &paths);
        assert_eq!(index.status, 200);
        assert!(index.body.contains("app"));

        let asset = handle_request("GET /assets/app.js HTTP/1.1\r\n\r\n", &config, &paths);
        assert_eq!(asset.status, 200);
        assert_eq!(asset.content_type, "text/javascript; charset=utf-8");
        assert!(asset.body.contains("vue"));

        fs::remove_dir_all(root).ok();
    }
}
