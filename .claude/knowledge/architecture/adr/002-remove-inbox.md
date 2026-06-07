# ADR 002: 移除 inbox 概念，library 原地索引

**日期**：2026-05  
**状态**：已采纳

## 背景

原始设计有两阶段文件流：`inbox/`（待处理暂存区）→ `process_file()`（移动+索引）→ `library/`（已索引存储区）。用户需要两步操作，体验复杂。

## 决策

**移除 inbox 概念**。用户直接将文件放入 `library/` 目录（或其 public/private 子目录），watch 监听器检测到文件后原地索引，不再移动文件。

## 实施方案

1. `fs_layout.rs`：移除 `pub inbox: PathBuf`
2. `config.rs`：移除 `default_inbox()` 和 `PathsConfig.inbox`
3. `processor.rs`：新增 `index_file_in_place()`——不移动文件，直接提取+分类+写入 DB
4. `watch.rs`：调用 `index_file_in_place()` 替代 `process_file()`
5. `handle_remove()`：删除文件时同步删除 DB 记录
6. 前端 `ConfigView.vue`：移除 inbox 路径字段

## 后果

- 架构大幅简化：文件生命周期从两步变一步
- 用户可直接管理 library 目录结构
- 删除文件自动清理 DB 记录
