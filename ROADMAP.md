# OmniOwn 路线图

当前状态：**两阶段 AI 搜索管线已完成 → 打磨搜索体验与跨平台发布**。

---

## 📌 已完成

### ✅ MCP Server

MCP（Model Context Protocol）stdio 服务器已实现，支持任意 MCP 兼容客户端。

| 工具 | 参数 | 说明 |
|------|------|------|
| `search_documents` | `query` (必填) | FTS5 全文搜索 |
| `get_document` | `id` (必填) | 获取文档完整内容 |
| `list_documents` | `folder_type`、`limit` | 列出文档摘要 |
| `get_status` | 无 | 知识库统计 |

`cargo run -- mcp` 启动，stdin/stdout JSON-RPC 2.0，MCP 2025-03-26 协议。

### ✅ PDF / Office 文档解析

支持 PDF、docx、pptx、xlsx 四种格式的文本提取。

| 格式 | 方案 | 状态 |
|------|------|------|
| PDF | `lopdf` 提取全部页面文本 | ✅ |
| xlsx | `calamine` 遍历所有 sheet 单元格 | ✅ |
| docx | `zip` + `quick-xml` 解析 `w:t` 文本 | ✅ |
| pptx | `zip` + `quick-xml` 解析 `a:t` 文本 | ✅ |

### ✅ 剥离向量化（embedding）

移除 ~1500 行 embedding 代码，项目更轻量。`ai-search`（LLM→搜索词→FTS）完全替代语义搜索。

### ✅ 两阶段 AI 搜索管线

- **Stage 1 查询分析** — LLM 改写查询 + 提取关键词/意图/分类/文件类型/时间范围/隐私偏好。失败时自动降级使用原始查询。
- **Stage 2 策略选择 + JSON Schema 验证** — LLM 从 8 策略中选择最优组合，zod 验证输出合法性。
- **Prompt 模块化** — v1/v2 变体 A/B 测试，v2 含 Few-shot 示例 + 文档库统计上下文注入。
- **分层结果合并** — FTS 命中全保留，非 FTS 最多 5 条补充，噪声大幅降低。
- **文档统计缓存** — 60 秒 TTL，导入/监听事件触发失效。

---

## 中期（3-5 个迭代）

### 🔍 搜索体验打磨

- AI 搜索流式输出（SSE），实时显示推理过程
- FTS5 中文分词优化（集成 ICU tokenizer）
- `search` CLI 支持 `--json` 输出（方便脚本和工具链集成）

### 🧹 Doctor 自修复

- `doctor` 增加 `--fix` 模式：检测孤立 DB 记录、缺失文件、损坏索引
- 自动修复常见不一致问题

### ⚙️ 跨平台发布

- GitHub Releases 自动构建（Linux + macOS + Windows）
- 提供预编译二进制下载
- CI 增加集成测试（含 UI 构建验证）

---

## 长期（未来）

### 🖥️ Tauri 桌面壳

将 Vue UI 打包为独立桌面应用，提供系统托盘、开机启动、通知等原生体验。

### ☁️ 可选同步

WebDAV / Syncthing 集成，多设备间同步知识库（始终本地优先）。

---

## 当前不计划

- OCR / 图片理解 — 超出纯文本知识库定位
- 云同步服务端 — 保持本地优先，不建云服务
- 多人协作 — 个人工具
