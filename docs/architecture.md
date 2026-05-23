# 架构文档

## 总体数据流

```
inbox/
  │
  ▼
file watcher (notify)
  │  Create / Modify / Remove 事件分流
  │  Modify 事件防抖 (1s 窗口)
  │  类型过滤 (白名单扩展名)
  ▼
processor::process_file()
  │  1. 读取文件内容
  │  2. 计算 SHA256 hash
  │  3. classifier 分类 (public/private)
  │  4. storage 生成存储路径
  │  5. 写入 library/{public|private}/
  │  6. db::upsert_document (SQLite)
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
library/{public|private}/{date}_{hash8}_{safe_filename}
```

例如：

```
library/public/2026-05-22_a81f39c2_rust_note.md
library/private/2026-05-22_bbbbbbbb_secret.md
```

**注意：**

- 文件物理路径不再细分文档类型（所有文档类型信息保存在数据库 `category` 字段）
- `public` / `private` 当前是逻辑目录分类，**不是加密隔离**
- 文件名经过清理（移除路径分隔符），空文件名默认 `unnamed`
- 存在同名文件冲突时自动追加序号（`_1`, `_2`, ...）

## 模块说明

| 模块 | 职责 |
|------|------|
| `src/main.rs` | 入口：CLI 命令分派 + 哨兵主循环（Tokio 异步） |
| `src/config.rs` | TOML 配置文件加载 + 环境变量覆盖 |
| `src/db.rs` | SQLite CRUD：文档管理、FTS5 全文检索、Embedding CRUD、统计查询 |
| `src/migration.rs` | Schema 迁移框架：幂等迁移、版本追踪 |
| `src/embedding.rs` | Embedding Provider 抽象（trait）+ Mock 实现 + Local stub + 向量工具 |
| `src/embedding_worker.rs` | 空闲 Embedding Worker：ActivityTracker、配置、非重入 |
| `src/classifier.rs` | 文本分类：基于关键词的 public/private 分类 + 类型标签 |
| `src/doctor.rs` | 系统健康检查 + 状态概览输出 |
| `src/fs_layout.rs` | 文件系统目录结构定义与初始化 |
| `src/processor.rs` | 文件处理管线：读取 → 分类 → 存储 → 入库 |
| `src/storage.rs` | 文件存储路径生成：日期 / hash / 防冲突 |
| `src/tests.rs` | 集成测试（100 文件批量导入等） |
