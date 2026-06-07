# bug-fix — Bug 修复

## 触发条件
- 用户说"修复 X bug"、"X 报错"、"X 不工作"、"X 异常"

## 流程

### 1. 复现
- 写出复现步骤或复现测试用例
- 明确症状（错误消息、日志、堆栈）

### 2. 定位根因（不修症状）
- 跟踪完整调用链（三层架构：Tauri → Node.js → Rust CLI）
- 通过日志/断点/bisect 定位精确失败点
- 特别关注跨进程边界：env vars 传递、CLI args 传递、路径解析

### 3. 最小修复
- 不顺便重构无关代码
- 涉及 Rust + TypeScript 双端时，确保两端契约一致
- 跨进程问题优先用 env vars 传递配置

### 4. 回归测试
- 添加复现该 bug 的测试用例
- 运行 `cargo test` 全部通过

### 5. 记录
- 非显而易见的坑写入 `knowledge/lessons-learned/`
- 更新 `troubleshooting/common-issues.md`

## 常见根因模式
- 路径解析不一致（相对路径 `./` 在不同 CWD 下行为不同）
- 跨进程配置丢失（env vars 未传递）
- 文件写入未完成就触发索引（时序竞态）
- SQLite journal 模式冲突（多进程访问）
