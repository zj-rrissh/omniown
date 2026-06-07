# API 文档 v1

## 通用约定

- **Base URL**：`http://127.0.0.1:3001`
- **Content-Type**：`application/json`
- **错误响应**：`{ "error": { "message": "..." } }`，HTTP 状态码 4xx/5xx

## 端点列表

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/status` | 系统状态统计 |
| GET | `/api/documents` | 文档列表（最近 20 条，不含 content） |
| GET | `/api/documents/:id` | 文档详情（含 content） |
| GET | `/api/search?q=&ai=` | 全文搜索 / AI 搜索 |
| GET | `/api/config` | 读取配置 |
| PUT | `/api/config` | 保存配置 |

## 端点详情

### GET /api/status

```json
{
  "database": "omniown.db",
  "root": "data",
  "schema": { "current_version": 5, "pending_migrations": 0 },
  "documents": {
    "total": 42, "public": 30, "private": 12,
    "indexed": 40, "failed": 2
  }
}
```

### GET /api/documents

返回最近更新的 200 条文档摘要（不含 content 字段），`id` 倒序。

### GET /api/documents/:id

返回单条文档完整信息，包含 content 字段（纯文本）。

### GET /api/search?q=关键词&ai=true

- 无 `q` 参数：返回文档列表
- `q` + 无 `ai`：FTS5 全文搜索
- `q` + `ai=true`：AI 多策略搜索（需配置 `[ai]` 节）
- 返回格式：`{ "results": [...], "total": N, "strategy": "fts5"|"ai" }`

### GET /api/config

返回当前 TOML 配置，含脱敏的 api_key（仅显示前 4 位 + `***`）。

### PUT /api/config

请求体：`{ "ai": { "base_url": "...", "model": "...", "api_key": "..." }, "paths": { "root": "...", "library": "..." } }`

保存后自动重启 Node.js 和 MCP 进程以应用新配置。
