# 问题追踪与解决方案

> 记录开发过程中遇到的典型问题、根因分析和解决方案，防止遗忘。

---

## 1. 数据库分离（split-brain）

**症状：** 前端页面不显示新导入的文件。watch 进程正常检测并导入，但 API 返回空。

**调用链：**
```
watch → process_file → rusqlite::Connection::open("./dev.db")
                                      → CWD 解析 → server/dev.db
Prisma → DATABASE_URL=file:./dev.db
       → 相对于 schema.prisma 解析 → server/prisma/dev.db
结果：两个不同的数据库文件
```

**根因：** `file:./dev.db` 中 `./dev.db` 是相对路径。Prisma 以 schema 文件位置为基准解析，Rust CLI 以 CWD 为基准解析，导致写入和读取是两个不同的数据库文件。

**解决方案：** `resolveDbPath()` 将 `file:./dev.db` 解析为绝对路径再传给 `--db-path`：
```typescript
if (!path.isAbsolute(dbPath)) {
  dbPath = path.resolve(projectRoot, 'prisma', dbPath)
}
```

**关联文件：** `server/src/index.ts:resolveDbPath()`, `src/main.rs:resolve_watch_db_path()`

---

## 2. SQLite 并发访问损坏

**症状：** `database disk image is malformed` 错误，Prisma 无法读取数据库。

**根因：** Prisma 默认使用 DELETE journal 模式，Rust 的 db::init_database 切换为 WAL 模式。两个进程同时打开数据库时，journal mode 切换导致 Prisma 连接持有的文件描述符看到不一致的数据库状态。

**解决方案：** Prisma db push 后立即通过 sqlite3 CLI 执行 `PRAGMA journal_mode=WAL`，确保所有后续连接（Prisma 和 rusqlite）都使用 WAL 模式。

**关联文件：** `server/src/index.ts`（db push 后的 PRAGMA），`src/db.rs:init_database()`

**验证：** WAL 模式允许一写多读并发，不再出现 disk I/O error。

---

## 3. 文件未写完就被导入（空 content）

**症状：** 导入的文件在数据库中有记录，但 content 字段为空字符串，file_hash = `e3b0c44298...`（空字符串 SHA256）。手动调用 extract 能正常提取内容。

**调用链：**
```
用户复制文件到 inbox
  → notify 触发 Create 事件（文件可能只有 0 字节）
  → watch 立即调用 process_file
  → extract_text 读取 0 字节文件 → content=""
  → db::upsert_document → (content="", file_hash="e3b0c4...")
  → debounce 阻止后续 Modify 事件重新导入
  → content 永远为空
```

**根因：** notify 的 Create 事件在文件写入完成前触发。原实现对第一个事件立即处理，debounce 反而阻止了后续重新导入。

**解决方案：** 改为文件稳定性检测：
1. 所有 Create/Modify 事件只记录到 `pending` 队列（路径 + 时间 + 大小）
2. 每 500ms 轮询 `pending`：文件 1 秒内无新事件 且 当前大小 = 记录大小 → 判定稳定
3. 稳定后才调用 `process_file`

```
事件 → pending 队列 → 1s 无变化 + 大小稳定 → process_file
```

**关联文件：** `src/watch.rs`（事件循环 + PendingFile 结构）

---

## 4. 启动时已有文件不被导入

**症状：** inbox 中在服务启动前就存在的文件不会被导入，watch 只处理启动后的新文件。

**根因：** watch 是纯反应式监听，notify 只报告注册后发生的事件。启动前已存在的文件不会触发任何事件。

**解决方案：** 在 watcher.watch() 之前增加初始扫描步骤，遍历 read_dir(inbox)，逐个调用 process_file。process_file 内部通过 file_hash 去重，已导入的同名文件会被跳过。

**关联文件：** `src/watch.rs:run_watch()` 步骤 2.5

---

## 5. capabilities/ 目录被 gitignore 忽略

**症状：** `src-tauri/capabilities/default.json` 无法提交到 git，CI 构建缺少权限声明。

**根因：** `.gitignore` 第 30 行 `src-tauri/capabilities/` 错误地将 Tauri v2 的用户配置目录标记为忽略。`gen/` 是自动生成的，但 `capabilities/` 是手写的权限配置，必须纳入版本控制。

**解决方案：** 从 `.gitignore` 中移除 `src-tauri/capabilities/`。

**关联文件：** `.gitignore:30`

---

## 6. tauri-plugin-dialog 跨平台后端

**症状：** `notify = { default-features = false, features = ["macos_kqueue"] }` 只在 macOS 上有文件监听后端，Linux/Windows 上 `recommended_watcher()` 失败。

**根因：** 禁用 default-features 后未显式添加 Linux/Windows 后端。

**解决方案：** 使用 `notify = "7"` 启用默认 features（自动平台检测，包含 inotify/ReadDirectoryChanges/kqueue）。

**关联文件：** `Cargo.toml`（Rust CLI）

---

## 6. 经验总结

### 跨进程数据库共享

- SQLite 多进程访问必须统一 journal_mode 为 WAL
- 相对路径在 `DATABASE_URL` 中应尽早解析为绝对路径，不同组件对相对路径的解析基准不同

### 文件监听

- notify 的 Create 事件在文件写入完成前触发
- 必须实现稳定性检测（等待文件大小不再变化 + 足够静默时间）
- 初始扫描和增量监听是互补的两个机制
- debounce 用于防重复，不是用于防提前触发

### Tauri v2 配置

- `capabilities/` 目录是手写配置，应纳入版本控制
- `gen/` 目录是自动生成的，保持 gitignore
- `tauri-plugin-dialog` 前端 JS 包和 Rust crate 需同时安装

### 开发流程

- 集成测试前清理 SQLite 的 WAL/SHM 文件（`rm -f dev.db*`）
- `sqlite3` CLI 是调试数据库问题的快速工具
- `strings` 命令可验证二进制中是否包含特定代码
