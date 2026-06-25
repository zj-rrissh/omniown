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

## 经验总结

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

### 配置管理

- 服务端 `saveConfig` 必须使用深度合并而非全量替换，避免非前端管理字段被静默清除
- 前端表单不应假设自己拥有配置文件的全部所有权，需设计字段合并策略
- 配置读写和 watch 配置路径应在编译期或启动时统一为绝对路径

### API 设计

- 列表接口的 `take/limit` 硬编码会静默截断数据，必须暴露分页参数并设置合理默认值
- 分页默认排序需谨慎：按 `updatedAt DESC` 排序 + 小 limit 会导致旧文档永远不可达
- 推荐设计：大默认 limit（如 200）+ 前端客户端分页 + 无限滚动懒加载

### 文件处理

- 文件移动（`rename`）后原路径 metadata 不可读，需在移动前捕获所需信息（大小、扩展名等）
- `stored_path_for_db` 等路径计算函数应统一为单一事实来源，避免跨模块手写 `strip_prefix`
- 提取失败降级策略：保留原始文件 + 记录已知字段，让用户可手动重试

### 开发流程

- 集成测试前清理 SQLite 的 WAL/SHM 文件（`rm -f dev.db*`）
- `sqlite3` CLI 是调试数据库问题的快速工具
- `strings` 命令可验证二进制中是否包含特定代码

---

## 7. 删除 library 文件后 DB 记录残留

**症状：** 用户在 library 中删除文件，前端搜索结果中仍能查到该文件，但文件已不存在。

**调用链：**
```
用户 rm library/public/file.md
  → 无代码监听 library → DB 中 file.md 记录不删除
  → API 返回该记录 → 前端展示 → 点击报 404
```

**根因：** `omniown watch` 原只监听 inbox，library 不在监听范围内。改为监听 library 后，`handle_remove` 中 `strip_prefix` 因 root 是相对路径而失败，导致 DB 记录无法删除。

**解决方案：** 
1. watch 改为递归监听 library 目录
2. `handle_remove` 中 root 转为绝对路径后再 `strip_prefix`
3. 稳定性检查中文件已消失时同步清理 pending + DB

**关联文件：** `src/watch.rs:run_watch()`, `src/watch.rs:handle_remove()`

> **后续修复（`7491438`）：** 后发现 `handle_remove` 中 `stored_path_for_db` 写入绝对路径而查询使用相对路径，导致路径前缀剥离后仍不匹配。最终改为统一调用 `processor::stored_path_for_db()` 计算路径，避免手动 `strip_prefix`。

---

## 8. process_file 在 library 内文件时触发冲突取消

**症状：** 文件已在 library/public/ 中，process_file 因目标路径已存在而 Cancel，不写入 DB。

**根因：** process_file 的 conflicting path check：`stored_path.exists() → Cancel`，导致已在 library 中的文件永远不会被索引。

**解决方案：** 
1. 新增 `index_file_in_place()` — 文件已在 library 中时跳过移动直接索引
2. 在 `process_file_with_conflict_decision()` 中增加 `is_in_place` 检测：源 == 目标时跳过冲突检查

**关联文件：** `src/processor.rs:index_file_in_place()`, `src/processor.rs:process_file_with_conflict_decision()`

---

## 9. saveConfig 全量替换导致非前端字段丢失

**症状：** 前端设置页保存配置后，`prompt_variant` 等非前端管理的配置字段被清空，AI 搜索行为异常。

**调用链：**
```
前端保存 [ai] + [paths] 段
  → server/src/config/index.ts:saveConfig()
  → 全量写入 TOML 文件（覆盖原有内容）
  → prompt_variant 等字段丢失
  → AI 搜索回退到默认 prompt 变体
```

**根因：** `saveConfig` 使用 `toml.stringify(obj)` 全量序列化覆盖写入。前端只发送 UI 中展示的字段（`[ai]` + `[paths]`），`prompt_variant` 等由其他途径（如 CLI）管理的字段不在前端表单中，保存后被清空。

**解决方案：** 改为读现有配置 → 深度合并 → 写入：
```typescript
const existing = parse(content)
const merged = deepMerge(existing, updates)
await fs.writeFile(configPath, stringify(merged))
```

**关联文件：** `server/src/config/index.ts:saveConfig()`

---

## 10. 文档列表 API 硬编码 limit 导致部分文档不可见

**症状：** 文档列表页显示 20 条记录，部分文档（如 `Web课程设计说明书.txt`）从未出现在列表中。重启服务、重新导入均无效。

**调用链：**
```
前端请求 /api/documents
  → server/src/api/documents.ts
  → prisma.document.findMany({ take: 20, orderBy: { updatedAt: 'desc' } })
  → 返回最新的 20 条
  → 更新时间较早的文档永远排在第 21 位之后
```

**根因：** API 硬编码 `take:20` 按 `updatedAt DESC` 排序，且未暴露分页参数。当文档总数超过 20 时，更新时间最早的文档被截断，前端无法通过任何操作访问到它们。

**解决方案：** 移除硬编码 limit，改为读取前端 `req.query.limit` 参数（前端传 `?limit=200`），同时支持 `?skip=` 偏移参数，前端 store 自行分页。

**关联文件：** `server/src/api/documents.ts`

---

## 11. 提取失败时 file_size 为空

**症状：** 文档提取失败（如加密 PDF）后，数据库记录中 `file_size` 字段为 `null`，状态页统计大小无法计算。

**调用链：**
```
process_file → extract_text 失败
  → handle_extraction_failure
  → std::fs::rename(original_path → stored_path)
  → 读取 metadata(stored_path) → 获取 file_size
  → 写入 DB 时 file_size = null
```

**根因：** `handle_extraction_failure` 先执行 `std::fs::rename` 将文件移动到目标路径，然后尝试从原路径读取 `metadata`。移动后原路径已不存在，`metadata()` 返回 `Err`，`file_size` 被设为 `null`。

**解决方案：** 在移动文件前捕获 `file_size`，移动后只从 `stored_path` 读取扩展名：
```rust
// rename 前捕获 size
let file_size = fs::metadata(&source_path).ok().map(|m| m.len() as i64);
// rename 后只取扩展名
let ext = stored_path.extension().and_then(|e| e.to_str());
```

**关联文件：** `src/processor.rs:handle_extraction_failure()`
