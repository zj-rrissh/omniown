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

### `omniown config-example`

输出配置模板到 stdout。

```bash
omniown config-example > omniown.toml
```

---

## ⚠️ 待实现：`omniown watch`

**当前状态：未实现。**

目标：文件监听子命令，随服务启动后台运行，监听 `inbox` 目录的新增文件并自动触发 `process`。

```bash
# 目标用法
omniown watch [--inbox <path>] [--library <path>]
```

**需求：**
- 基于 `notify` crate 实现跨平台文件系统事件监听
- 监听目录由 `omniown.toml` 的 `paths.inbox` 指定
- 检测到新文件时自动调用 `process`
- 支持配置变更后重载监听路径
- Node.js 启动时 spawn `omniown watch` 进程

---

## Node.js 集成

```typescript
// server/src/services/import.service.ts
import { exec } from 'child_process'

export function processFile(filePath: string): Promise<ImportResult> {
  return new Promise((resolve, reject) => {
    exec(`omniown process "${filePath}"`, (err, stdout, stderr) => {
      if (err) reject(new Error(stderr || err.message))
      else resolve(JSON.parse(stdout))
    })
  })
}
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
```
