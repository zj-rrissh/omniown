# Phase 1-6 全面审查报告

> 审查时间：2025-05-24  
> 审查范围：全部 Rust 后端、Tauri 壳、Vue 前端、CI 配置  
> 审查方式：3 个并行子 agent（安全审查 / 代码审查 / 性能架构审查）

---

## 🔴 CRITICAL — 阻断发布

| # | 问题 | 位置 | 风险 |
|---|------|------|------|
| 1 | **CSP 完全禁用 (`null`)** | `src-tauri/tauri.conf.json:59` | XSS → 任意 `invoke` 调用 → 写文件/窃取密钥/执行 sidecar |
| 2 | **API Key 明文存储 + IPC 无授权暴露** | `main.rs:106-124` `config.rs:53` | `read_config` 返回完整 `api_key` 给前端；`write_config` 无校验可被任意改写 |
| 3 | **HTTP API 零认证** | `src/ui_server.rs:47-48,139-147` | `127.0.0.1:17777/api/documents/1` 直接返回 private 文档全文，绑定 `0.0.0.0` 后局域网可达 |
| 4 | **Sidecar 孤儿进程** | `src-tauri/src/main.rs:180-210` | Tauri 被 `SIGKILL`/崩溃/`taskkill /F` 时 `omniown serve` 子进程残留在系统 |
| 5 | **单线程 HTTP 服务器 → DoS** | `src/ui_server.rs:82-93` | `TcpListener::incoming()` 同步处理，大文件或慢客户端阻塞所有后续请求 |
| 6 | **MCP 开关是假功能** | `src-tauri/src/main.rs:151-154` | `toggle_mcp()` 只在内存翻转 `bool`，从未 spawn/kill `omniown mcp` 进程 |
| 7 | **配置文件路径硬编码相对路径** | `src-tauri/src/main.rs:174` | `PathBuf::from("../config/omniown.toml")` 相对 CWD，生产环境从任意目录启动找不到配置文件 |

---

## 🟠 HIGH — 发布前修复

| # | 问题 | 位置 | 修复方向 |
|---|------|------|---------|
| 8 | **Sidecar 崩溃无限重启无退避** | `main.rs:226-237` | 加指数退避 + 最大重试次数，防 fork-bomb |
| 9 | **FTS5 trigger 非原子操作** | `src/migration.rs:147-148` | delete + insert 之间崩溃导致 FTS 索引永久丢失，改用事务包裹 |
| 10 | **`is_cross_device_error` Windows 不兼容** | `src/processor.rs:152` | `EXDEV` 硬编码 18，Windows 是 17，`#[cfg]` 条件编译 |
| 11 | **配置损坏静默降级** | `main.rs:46` `config.rs:117` | `unwrap_or_default()` 吞掉所有解析错误，应返回 `Err` |
| 12 | **`tests-config/` 纯代码复制** | `tests-config/src/main.rs:34-62` | 完全复制 `read_ai_config`/`write_ai_config`，修改 Tauri 端后不同步失控 |
| 13 | **详情面板代码重复** | `SearchView.vue` `DocumentsView.vue` | 各自实现一套 `detail-panel`，抽取为共享 `DocumentDetailPanel.vue` |
| 14 | **客户端分页拉全部数据** | `DocumentsView.vue:27` | `fetchDocuments(200)` 每次请求全量，超 200 篇后遗漏 |
| 15 | **Cargo.toml Edition 2024** | 根 `Cargo.toml:3` | 需 nightly Rust，稳定版无法编译 |
| 16 | **SQLite 连接未池化** | ~10 处 `Connection::open` | 每请求/每文件开新连接，WAL 初始化开销重复 |

---

## 🟡 MEDIUM — 后续版本修复

| # | 问题 | 位置 | 修复方向 |
|---|------|------|---------|
| 17 | 预编译 sidecar 二进制驻留 Git | `binaries/omniown-*` | 供应链风险，CI 中构建而非提交二进制 |
| 18 | API 响应泄露内部路径 | `ui_server.rs:283-289` | `/api/status` 返回绝对路径，改为相对或摘要 |
| 19 | `classifier.rs` O(n²) 字符串 | `classifier.rs` | 大文件 lowercase 副本 + `contains()` 扫描，逐行处理 |
| 20 | AI API 无超时/断路器 | `src/ai.rs:59` | `reqwest::Client::new()` 默认 30s，失败后无退避 |
| 21 | Vue 无代码分割 | `router.ts` | 4 组件全部静态导入，单 bundle，加 `defineAsyncComponent` |
| 22 | 日志含敏感文件名 | `processor.rs:268` | `imports.log` 记录原始路径，可能含敏感信息 |
| 23 | 详情面板无内容截断 | `SearchView.vue:62` | 数 MB 文本直接渲染，可能 OOM |
| 24 | API 客户端无重试/Abort | `api.ts` | 切换视图时悬挂请求，sidecar 重启后不重试 |
| 25 | `remove_tag_blocks` 缺闭合标签丢内容 | `extractor.rs:164-167` | `<script>` 无 `</script>` 时后续正文全丢弃 |

---

## 🔵 NIT — 代码质量优化

| # | 问题 | 位置 |
|---|------|------|
| 26 | `bootstrap()` 重复读取 `OMNIOWN_ROOT` | `main.rs:103` + `config.rs:73` |
| 27 | `search_documents_filtered` 参数编号脆弱 | `db.rs:284-298` |
| 28 | 多个组件用不同 CSS 类名实现相同结构 | `.result-row` vs `.doc-row` |
| 29 | 手动 JSON 序列化函数重复 | `ui_server.rs:281-313` — 重复 serde_json 已有功能 |
| 30 | Manual HTTP 服务器 400 行 | `ui_server.rs` — `axum` 或 `actix-web` 可消除大部分手写解析 |

---

## ✅ 确认安全无问题

| 模块 | 检查项 |
|------|--------|
| `db.rs` 所有 SQL | ✅ 参数化绑定，无拼接注入 |
| `storage.rs` | ✅ 文件名清洗 `/` `\` `\0` → 阻止目录穿越 |
| `extractor.rs` | ✅ 只读文件 + UTF-8 编码安全 |
| `mcp.rs` JSON-RPC | ✅ 协议全覆盖，错误恢复正确 |
| `migration.rs` | ✅ DDL 事务保护、回滚机制 |
| `ai.rs` API 调用 | ✅ `Authorization: Bearer` 通过 header 非 URL |
| `config.rs` 路径解析 | ✅ 相对路径基于 root 拼接 |
| CI pipeline | ✅ 无凭据泄漏 |

## 测试覆盖统计

| 模块 | 测试数 | 状态 |
|------|:------:|------|
| `src/` 后端 (265 tests) | 265 | ✅ 全部通过 |
| `src/mcp.rs` | 全覆盖 | ✅ |
| `src-tauri/` 纯逻辑 | 13 | ✅ 全部通过 (独立测试框架) |
| `src/ai.rs` | 序列化仅 | ⚠️ 缺 HTTP mock 集成测试 |
| `src/main.rs` watcher | 仅 guard 测试 | ⚠️ 缺 notify 事件集成测试 |
| Vue 前端 | 0 | ❌ 无 vitest 配置 |
| Sidecar 集成 | 0 | ❌ 无 Tauri 集成测试 |

---

## 修复优先级建议

```
第一轮（阻断发布）: #1-#7  CRITICAL
第二轮（发布前）:   #8-#16 HIGH
第三轮（v0.1.1）:   #17-#25 MEDIUM
后续：              #26-#30 NIT
```
