# CLI 命令

Rust Core + CLI 提供重型文件处理能力。`omniown_core` 是可复用 Rust 内核，`omniown` 二进制是兼容 CLI 入口，由 Node.js 后端通过 `child_process.exec` / `spawn` 调用。

推荐 Rust 外部项目优先依赖 `omniown_core::runtime::OmniownKernel`。非 Rust 项目继续调用本页命令。

---

## 子命令

### `omniown process <path>`

导入文件到知识库。文件已在 `library/` 目录下时原地索引，不移动文件。

```bash
omniown process library/public/note.md
omniown process ~/Downloads/report.pdf
```

**处理流程：**

```
文本提取 → 分类（公开/私密/类别/风险）→ 原地索引（不移动文件）→ 写入 SQLite
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

### `omniown watch [--db-path <path>] [--library <path>]`

启动文件夹监听，递归监听 `library` 目录，文件增删自动同步数据库。

```bash
omniown watch
omniown watch --db-path /path/to/dev.db --library /path/to/library
```

**数据库路径优先级：** `--db-path` CLI 参数 > `DATABASE_URL` 环境变量 > `omniown.toml` 默认值。

**行为：**
- 基于 `notify` crate 跨平台递归监听 library 目录
- 文件新增 → 原地索引（extract → classify → upsert，不移动文件）
- 文件删除 → 自动清理数据库记录
- 启动时递归扫描 library 已有文件
- stdout 首行输出 JSON 就绪信号：`{"status":"watching","library":"<path>","db_path":"<path>"}`
- 文件稳定性检测（1s 无变化 + 大小不变）后处理，避免未写完文件
- 自动过滤临时文件（`.tmp` / `.crdownload` / `.part` / `~$` / 隐藏文件）
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
const child = spawn('omniown', ['watch', '--db-path', dbPath, '--library', libraryPath])
child.stdout.on('data', (data) => {
  const info = JSON.parse(data.toString()) // { status: "watching", library: "...", db_path: "..." }
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
