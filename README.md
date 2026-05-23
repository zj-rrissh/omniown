# OmniOwn

[![CI](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml/badge.svg)](https://github.com/zj-rrissh/omniown/actions/workflows/ci.yml)

**Local-first, privacy-first, offline-by-default personal document/knowledge-base backend.**

OmniOwn 是一个纯 Rust CLI 本地文档管理后端。它监控一个 `inbox` 目录，自动导入文本类文件，提取正文与元数据，建立全文索引（FTS5），并通过可插拔的 embedding provider 支持语义搜索骨架。

> ⚠️ **当前状态：early backend prototype**
>
> - 没有 UI / Tauri 界面
> - 没有生产级真实语义模型（local provider 只有 feature-gated token-hash 实验实现）
> - `private` / `public` 是逻辑目录分类，不等于加密
> - 适合本地开发与测试，尚未达到生产发布标准

---

## 当前能力

- **文件监听** — 通过 `notify` 监控 `inbox` 目录，支持 Create / Modify / Remove 事件分流
- **自动导入** — 支持的文本类文件导入后按规则存入分层目录
- **文本提取** — 统一 extractor 管线，支持纯文本、Markdown、HTML、代码、常见配置/数据文件
- **Hash 去重** — SHA256 内容哈希检测，内容未变则跳过
- **分层存储** — 文件按 `public` / `private` 分类存储：

  ```
  library/{public|private}/{date}_{hash8}_{safe_filename}
  ```

- **SQLite 元数据存储** — 文档信息、分类、标签、处理状态持久化
- **FTS5 全文搜索** — SQLite FTS5 虚拟表，实时同步，支持 snippet
- **Mock Embedding** — 确定性 hash 向量，用于开发和离线测试
- **语义搜索骨架** — 基于 embedding 的向量相似度搜索
- **Lazy Idle Embedding Worker** — 空闲时段批量计算 embedding
- **Config 系统** — TOML 配置文件 + 环境变量覆盖
- **Schema Migration** — 数据库版本管理，幂等可重复执行
- **Doctor / Status** — 系统健康检查与状态概览
- **Model-aware Embedding** — 复合主键 `(document_id, model_name)` 支持多模型共存

## 暂不支持

- UI / Tauri 界面
- 云同步 / 多设备
- OCR / 图片理解
- PDF / Office 文档解析
- 真正的本地 embedding 模型
- 向量数据库
- 加密 private 存储
- 网络请求

---

## 快速开始

### 构建与测试

```bash
cargo build
cargo test
```

### 查看状态

```bash
cargo run -- doctor
cargo run -- status
```

### 启动哨兵

```bash
cargo run
```

程序会监听 `./inbox` 目录，将新文件自动导入处理。

当前支持的导入扩展名：

```text
txt, md, markdown, html, htm,
rs, js, ts, jsx, tsx, py, java, go, cpp, c, h, hpp, css, sh, sql,
json, toml, yaml, yml, csv, log
```

### 导入文件

```bash
echo "hello rust async queue" > inbox/test.md
cargo run
```

### 搜索

```bash
cargo run -- search rust
cargo run -- search "async queue"
```

### 语义搜索（mock embedding）

```bash
cargo run -- embedding-provider-info
cargo run -- embed --provider mock
cargo run -- semantic-search "rust async queue" --provider mock
```

### local provider 测试

```bash
cargo run -- embed --provider local
cargo run --features local-embedding -- embed --provider local
```

默认构建下 local provider 是 stub，应清晰报错退出，不应 panic。开启
`local-embedding` feature 后，local provider 会使用离线 token-hash 实验实现跑通
embedding pipeline。

---

## 项目结构

```
OmniOwn/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口 + CLI 分派 + 哨兵主循环
│   ├── config.rs            # TOML 配置加载
│   ├── db.rs                # SQLite CRUD / FTS5 全文检索
│   ├── migration.rs         # Schema 迁移系统
│   ├── embedding.rs         # Embedding Provider trait / Mock / Local stub
│   ├── embedding_worker.rs  # 空闲 Embedding Worker
│   ├── extractor.rs         # 文本提取与支持格式白名单
│   ├── classifier.rs        # 文本分类
│   ├── doctor.rs            # 系统健康检查
│   ├── fs_layout.rs         # 文件系统目录规划
│   ├── processor.rs         # 文件处理管线
│   ├── storage.rs           # 文件存储路径生成
│   └── tests.rs             # 集成测试
├── config/
│   └── config.toml          # 用户配置（可选）
├── inbox/                   # 监控目录
├── library/
│   ├── public/              # 公开文件存储
│   └── private/             # 私有文件存储
├── index/
│   └── omniown.db           # SQLite 数据库
└── docs/                    # 文档
```

---

## 技术栈

| 组件 | 选型 |
|------|------|
| 语言 | Rust (edition 2024) |
| 异步 | Tokio |
| 文件监控 | notify |
| 数据库 | SQLite via rusqlite (bundled) |
| 全文检索 | FTS5 |
| Embedding | 可插拔 Provider 架构 |
| 配置 | TOML + 环境变量 |
| 序列化 | serde |
| 哈希 | SHA256 |
| 日期 | chrono |
