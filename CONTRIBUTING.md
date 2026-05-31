# Contributing to OmniOwn

感谢你的贡献兴趣！这份指南说明如何参与开发。

## 环境搭建

### 必需工具

- **Rust** (stable, edition 2024) — https://rustup.rs
- **Node.js** (>= 20) — https://nodejs.org
- **npm** (随 Node.js 安装)

### 克隆并安装

```bash
git clone https://github.com/zj-rrissh/omniown.git
cd omniown

# Rust
cargo build

# Node.js API
npm --prefix server install
npm --prefix server run build

# Vue 前端
npm --prefix ui install
```

## 开发流程

### 分支策略

1. 从 `main` 创建功能分支：`git checkout -b feat/your-feature`
2. 开发 + 提交
3. 推送分支：`git push origin feat/your-feature`
4. 创建 Pull Request 到 `main`

### 本地验证

提交前务必运行全部检查：

```bash
# Rust
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings

# Node.js API
npm --prefix server run build

# Vue 前端
npm --prefix ui run build

# Tauri 桌面
cargo test --manifest-path src-tauri/Cargo.toml
```

推荐使用项目内置的 `pr-ready` 检查脚本（自动执行上述 Rust 检查）。

## 代码规范

### Rust

- **Edition 2024**
- 测试写在源文件内：`#[cfg(test)] mod tests { … }`，不单独建 `tests/` 目录
- `cargo fmt` 格式化（CI 强制检查）
- `cargo clippy -- -D warnings` 零警告（CI 强制检查）
- 不提交 `dbg!()` / `todo!()` 等调试代码

### TypeScript (server/)

- **Strict mode** — `tsconfig.json` 中 `"strict": true`
- **ESM** — `"type": "module"`，import 使用 `.js` 扩展名
- **模块解析** — `NodeNext`
- Prisma v5（不升级到 v6/v7）
- 数据库字段 camelCase → `@map("snake_case")`

### TypeScript (ui/)

- **Strict mode** — `tsconfig.json` 中 `"strict": true`
- **Vite 构建** — `vue-tsc --noEmit && vite build`
- 组件使用 `<script setup lang="ts">` + Composition API
- 状态管理用 Pinia stores
- API 调用通过 `services/` 层

## 提交信息

建议使用中文、结构化格式：

```
<类型>：<一句话概括>

- <具体变更 1>
- <具体变更 2>
```

| 类型 | 场景 |
|:---|:---|
| `feat` | 新功能 |
| `fix` | 修复 bug |
| `refactor` | 重构 |
| `docs` | 文档 |
| `chore` | 构建/工具 |
| `test` | 测试 |

## 项目架构

三层全栈，理解各层职责有助于定位改动位置：

| 层 | 技术 | 职责 |
|:---|:---|:---|
| `src/` | Rust | 文本提取(PDF/DOCX/XLSX)、文件导入、MCP Server |
| `server/` | Node.js/TS | REST API、Prisma ORM、FTS5 搜索、AI 搜索 |
| `ui/` | Vue 3/TS | 搜索/文档/配置/状态四标签 UI |
| `src-tauri/` | Tauri v2 | 桌面壳：托盘 + 悬浮面板 + 子进程管理 |

## Pull Request 流程

1. 确保本地检查全部通过
2. 如果新增功能，添加测试
3. 如果变更 API，更新 `docs/` 中相关文档
4. PR 描述中说明：做了什么、为什么、如何验证
5. CI 全绿后请求审查

## 问题反馈

- Bug → 使用 Bug Report 模板提交 Issue
- 功能建议 → 使用 Feature Request 模板
- 问题讨论 → 在 Issue 中描述场景和期望
