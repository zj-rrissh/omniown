# 架构文档

## 总体数据流

```
inbox/
  │
  ▼
file watcher (notify)
  │  Create / Modify / Remove 事件分流
  │  Modify 事件防抖 (1s 窗口)
  │  类型过滤 (extractor 白名单扩展名)
  ▼
processor::process_file()
  │  1. extractor 提取正文
  │  2. 计算正文 SHA256 hash
  │  3. classifier 分类 (public/private)
  │  4. storage 生成存储路径 (保留原文件名)
  │  5. 同名冲突处理 → 交互终端提示覆盖/取消 (非交互默认取消)
  │  6. 写入 library/{public|private}/
  │  7. db::upsert_document (SQLite)
  ▼
SQLite documents 表
  │
  ├── FTS5 全文索引 (triggers 实时同步)
  │
  └── Lazy Idle Embedding Worker
        │  导入时不立即 embedding
        │  空闲时小批量处理
        ▼
      document_embeddings 表
        │
        ▼
      semantic-search / search
```

## 运行时目录结构

```
OmniOwn/
├── inbox/          ← 监控目录：放入文件即触发导入
├── library/
│   ├── public/     ← 公开文档存储
│   └── private/    ← 私有文档存储
├── index/          ← SQLite 数据库
├── cache/          ← 临时缓存
├── logs/           ← 日志文件
├── quarantine/     ← 处理失败的文件隔离区
├── trash/          ← 删除的文件暂存
└── config/         ← TOML 配置文件
```

### 文件存储规则

```
library/{public|private}/{safe_filename}
```

例如：

```
library/public/rust_note.md
library/private/secret.md
```

**注意：**

- 文件物理路径不再细分文档类型（所有文档类型信息保存在数据库 `category` 字段）
- `public` / `private` 当前是逻辑目录分类，**不是加密隔离**
- 文件名经过清理（移除路径分隔符），空文件名默认 `unnamed`
- 存在同名文件冲突时，交互终端提示覆盖或取消；非交互环境默认取消

## 模块说明

| 模块 | 职责 |
|------|------|
| `src/main.rs` | 入口：CLI 命令分派 + 哨兵主循环（Tokio 异步） |
| `src/config.rs` | TOML 配置文件加载 + 环境变量覆盖 |
| `src/db.rs` | SQLite CRUD：文档管理、FTS5 全文检索、Embedding CRUD、统计查询 |
| `src/migration.rs` | Schema 迁移框架：幂等迁移、版本追踪 |
| `src/embedding.rs` | Embedding Provider 抽象（trait）+ Mock 实现 + Local stub + 向量工具 |
| `src/embedding_worker.rs` | 空闲 Embedding Worker：ActivityTracker、配置、非重入 |
| `src/extractor.rs` | 文本提取：统一支持格式白名单，Markdown/HTML 轻量正文提取 |
| `src/classifier.rs` | 文本分类：基于关键词的 public/private 分类 + 类型标签 |
| `src/doctor.rs` | 系统健康检查 + 状态概览输出 |
| `src/fs_layout.rs` | 文件系统目录结构定义与初始化 |
| `src/processor.rs` | 文件处理管线：提取 → 分类 → 存储 → 入库 |
| `src/storage.rs` | 文件存储路径生成：保留原文件名并清理危险路径字符 |
| `src/ui_server.rs` | 本地只读 Web UI 与 JSON API |
| `src/tests.rs` | 集成测试（100 文件批量导入等） |

## 本地浏览 UI

`serve` 命令启动标准库实现的轻量 HTTP 服务，默认监听
`127.0.0.1:17777`。服务托管 `ui/dist` 中的 Vue + TypeScript/Vite 构建产物，
并提供只读 JSON API。它只读取 SQLite 元数据和 FTS 搜索结果，不改变导入、
迁移、embedding 或文件存储流程。

```text
Vue UI
  │
  ▼
ui_server
  │
  ├── GET /api/status       → db / migration / embedding 统计
  ├── GET /api/documents    → documents 元数据列表（不返回 content）
  ├── GET /api/search       → FTS5 search_documents
  └── GET /api/documents/id → 只读文档详情
```

前端源码位于 `ui/`，通过 `npm run build` 输出到 `ui/dist`；开发时可用
`npm run dev` 启动 Vite，并通过 `/api` proxy 访问后端。后端命令默认从项目根目录
运行，或通过 `OMNIOWN_ROOT` 显式指定数据根目录。v1 不提供写操作。

## 文本提取

`extractor` 是导入管线的格式入口，当前只处理 UTF-8 文本类文件：

```text
txt, md, markdown, html, htm,
rs, js, ts, jsx, tsx, py, java, go, cpp, c, h, hpp, css, sh, sql,
json, toml, yaml, yml, csv, log
```

- Markdown：轻量移除标题、列表、引用和常见行内标记，保留正文用于分类、FTS 和 embedding
- HTML：移除标签、`script` / `style` 内容，并解码常见 HTML entity
- 代码、配置、CSV、日志：按 UTF-8 文本原样导入

PDF、Office、OCR 后续可在 `extractor` 中增加专门实现，而不需要改动 watcher 或 processor 的主流程。
