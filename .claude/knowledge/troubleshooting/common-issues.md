# 常见问题索引

按症状查找解决方案。

| 症状 | 原因 | 解决方案 |
|------|------|---------|
| 前端显示"加载失败"/API 500 | 数据库表未创建 | 检查 `prisma db push` 是否执行成功，schema 路径是否正确 |
| 放入文件后前端不显示 | watch 未启动或监听错误目录 | 检查 `--library` 是否传入，library 路径是否正确 |
| "disk I/O error" | SQLite journal 模式冲突 | 确认 Node.js 启动时执行了 `PRAGMA journal_mode=WAL` |
| 文件放入后 content 为空 | 文件写入未完成就被索引 | 稳定性检测 1s 延迟，检查文件大小是否稳定 |
| 已有文件未被索引 | watch 只处理增量 | 确认 `scan_library()` 初始扫描执行 |
| 删除文件后数据库仍有记录 | Remove 事件处理失败 | 检查 `handle_remove` 的 root 路径 canonicalize |
| Linux watch 不响应删除 | notify 事件类型不匹配 | 需匹配 `Remove(RemoveKind::File)` 和 `Remove(RemoveKind::Any)` |
| CI clippy 报错 | 代码风格问题 | 运行 `cargo fmt && cargo clippy --fix -- -D warnings` |
| 打包后 Node.js 启动失败 | 二进制/配置路径错误 | 检查资源映射、`__dirname` 推算、环境变量注入 |
| 前端连接被拒 | Node.js 未启动或端口冲突 | 检查 3001 端口是否被占用，Node.js 进程日志 |

参见 `docs/troubleshooting.md` 获取详细调用链分析。
