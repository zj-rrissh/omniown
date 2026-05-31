# CLI 命令

所有 Rust 命令通过 `cargo run -- <command>` 执行。

## `cargo run -- process <file>`

导入单个文件到知识库。

```bash
cargo run -- process inbox/note.md
cargo run -- process ~/Downloads/report.pdf
```

流程：文本提取 → 分类（公开/私密）→ 存储到 `library/{public|private}/` → 写入 SQLite 数据库。

## `cargo run -- extract <file>`

提取文件纯文本内容到 stdout。

```bash
cargo run -- extract document.pdf
cargo run -- extract note.md
```

支持格式：TXT、Markdown、HTML、代码、JSON/YAML/TOML/CSV、PDF、DOCX、XLSX。

## `cargo run -- mcp`

启动 MCP Server，AI 客户端（Claude Desktop / Cursor）可直接连接本地知识库。

```bash
cargo run -- mcp
```

## `cargo run -- config-example`

输出配置模板到 stdout。

```bash
cargo run -- config-example > omniown.toml
```

---

## 启动本地服务

API 和前端已从 Rust 迁移至 Node.js + Vue。

### 开发模式

```bash
# 终端 1：API 服务
npm --prefix server run dev       # http://127.0.0.1:3001

# 终端 2：前端开发服务器
npm --prefix ui run dev           # http://127.0.0.1:5173
```

### 生产模式

```bash
npm --prefix server run build     # tsc → server/dist/
npm --prefix ui run build         # vite → ui/dist/
node server/dist/index.js         # 启动 API（port 3001）
```

## API 端点

| 方法 | 路径 | 说明 |
|:---|:---|:---|
| `GET` | `/api/status` | 系统状态 |
| `GET` | `/api/documents` | 文档列表 |
| `GET` | `/api/documents/:id` | 文档详情 |
| `GET` | `/api/search?q=` | FTS5 全文搜索 |
| `GET` | `/api/search?q=&ai=true` | AI 多策略搜索 |
| `GET` | `/api/config` | 读取配置 |
| `PUT` | `/api/config` | 更新配置 |
