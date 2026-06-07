# ADR 003: Node.js 作为中间层

**日期**：2026-05  
**状态**：已采纳

## 背景

需要决定 API 服务的实现方式：
- **方案 A**：Rust CLI 直接暴露 HTTP API（actix-web/axum）
- **方案 B**：Node.js Express 作为 API 中间层，调用 Rust CLI 处理重任务

## 决策

**选择方案 B（Node.js 中间层）**。理由：
1. Prisma ORM 生态成熟，Express 路由开发效率高
2. Rust CLI 保持单一职责（文件处理/监听/MCP），不引入 HTTP 复杂度
3. Node.js 管理 Rust CLI 子进程生命周期（watch 进程的启动/重启/日志）
4. 前端开发者更熟悉 Node.js 技术栈

## 实施方案

- Node.js Express 5 提供 6 个 REST 端点
- 通过 `child_process.spawn` 启动 `omniown watch`
- 通过 `child_process.execSync` 调用 `prisma` CLI
- Rust CLI 保持纯 CLI 接口（无 HTTP 依赖）

## 后果

- 增加了一个进程（Node.js）和一个运行时依赖
- 需要管理 Node.js 进程生命周期（Tauri spawn + auto-restart）
- 数据库访问分裂为两套代码（Prisma + rusqlite）→ 见 ADR 004
