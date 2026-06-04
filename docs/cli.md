# CLI 命令

Rust CLI 二进制 `omniown` 提供重型文件处理能力，由 Node.js 后端通过 `child_process.exec` 调用。

---

## 子命令

### `omniown process <path>`

导入单个文件到知识库。

```bash
omniown process inbox/note.md
omniown process ~/Downloads/report.pdf
```

**处理流程：**

```
文本提取 → 分类（公开/私密/类别/风险）→ 存储到 library/{public|private}/ → 写入 SQLite
```

### `omniown extract <path>`

提取文件纯文本内容到 stdout。

```bash
omniown extract document.pdf
omniown extract note.md
```

**支持格式：** TXT、Markdown、HTML、代码文件、JSON/YAML/TOML/CSV、PDF、DOCX、XLSX。

### `omniown mcp`

启动 MCP Server，AI 客户端（Claude Desktop / Cursor）可直接连接本地知识库。

```bash
omniown mcp
```

Tauri 桌面端通过 `toggle_mcp` 命令启停此进程。

### `omniown watch [--db-path <path>]`

启动文件夹监听，后台运行，监听 `inbox` 目录的新增文件并自动导入。

```bash
omniown watch
omniown watch --db-path /path/to/dev.db
```

**数据库路径优先级：** `--db-path` CLI 参数 > `DATABASE_URL` 环境变量 > `omniown.toml` 默认值。

**行为：**
- 基于 `notify` crate 跨平台文件系统监听
- 检测到新文件时自动调用 `process`（extract → classify → move → db upsert）
- stdout 首行输出 JSON 就绪信号：`{"status":"watching","inbox":"<path>","db_path":"<path>"}`
- 自动过滤临时文件（`.tmp` / `.crdownload` / `.part` / `~$` / 隐藏文件）
- 800ms debounce 去重，每 100 事件清理过期记录
- Node.js 服务启动时自动 spawn 此进程

### `omniown config-example`

输出配置模板到 stdout。

```bash
omniown config-example > omniown.toml
```

---

## Node.js 集成

**手动导入（import.service.ts）：**
```typescript
import { exec } from 'child_process'
exec(`omniown process "${filePath}"`, (err, stdout, stderr) => { ... })
```

**自动监听（index.ts）：**
```typescript
import { spawn } from 'child_process'
const child = spawn('omniown', ['watch', '--db-path', dbPath])
child.stdout.on('data', (data) => {
  const info = JSON.parse(data.toString()) // { status: "watching", inbox: "...", db_path: "..." }
})
```

---

## 开发模式

```bash
# 构建 CLI
cargo build

# 运行文件处理
cargo run -- process <file>

# 文本提取
cargo run -- extract <file>

# 启动 MCP Server
cargo run -- mcp

# 启动文件夹监听
cargo run -- watch --db-path /path/to/dev.db
```
