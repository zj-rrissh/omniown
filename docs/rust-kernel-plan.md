# Rust 内核化计划

## 背景

根 Rust crate 原先只有 `omniown` 二进制目标，`src/main.rs` 同时承担 CLI 参数分发和业务模块挂载。实际业务能力已经分布在 `config`、`db`、`extractor`、`fs_layout`、`processor`、`watch`、`mcp` 等模块中，适合抽成可复用内核。

本次内核化采用低风险路线：保留现有 `omniown` CLI 和 sidecar 打包方式，在同一个 crate 内新增 `omniown_core` library target。其他 Rust 项目可直接依赖库 API，Node.js、Tauri 和其他非 Rust 应用仍可通过 CLI 调用。

## 内核边界

`omniown_core` 是第一版稳定复用边界，公开两层接口：

| 层级 | 入口 | 说明 |
|:---|:---|:---|
| 推荐 API | `omniown_core::runtime::OmniownKernel` | 面向外部项目的门面，封装配置、路径和核心操作 |
| 低层模块 | `config` / `db` / `extractor` / `fs_layout` / `processor` / `watch` / `mcp` | 保留现有模块能力，优先供内核内部和高级集成使用 |

推荐外部项目优先使用 `OmniownKernel`，避免直接耦合低层模块细节。

## 公共 API

`OmniownKernel` 提供以下能力：

```rust
use omniown_core::runtime::OmniownKernel;

let kernel = OmniownKernel::load();
kernel.process_file(path)?;
let extracted = kernel.extract_text(path)?;
let result = kernel.index_file_in_place(path)?;
kernel.run_watch()?;
kernel.run_mcp()?;
```

加载方式：

- `OmniownKernel::load()`：沿用 CLI 默认行为，从 `OMNIOWN_ROOT` 或当前目录解析配置。
- `OmniownKernel::load_from_root(root)`：从指定数据根目录加载。
- `OmniownKernel::from_config(config)`：从已解析配置构造内核。
- `OmniownKernel::with_paths(config, paths)`：用于 CLI 或宿主应用覆盖路径。

## CLI 兼容性

`omniown` 二进制继续作为外部应用层的兼容入口，命令保持不变：

- `omniown process <path>`
- `omniown extract <path>`
- `omniown watch [--db-path <path>] [--library <path>]`
- `omniown mcp`
- `omniown config-example`

Node.js 后端继续通过 `child_process` 调用 `omniown`，Tauri 继续通过 `externalBin: ["binaries/omniown"]` 打包 sidecar。`watch` 的 stdout 首行 JSON ready 信号保持兼容。

## 更新策略

短期内，内核和 CLI 仍在同一个 crate 内发布，保证现有桌面端和服务端无需迁移。后续如果需要单独升级内核，可按以下顺序演进：

1. 先稳定 `OmniownKernel` 门面和文档。
2. 再将库迁移到 `crates/omniown-core`，CLI 迁移到 `crates/omniown-cli`。
3. 最后按需要拆分为独立仓库或发布到内部 registry。

第一版不引入 HTTP、FFI、动态库或独立仓库，避免扩大集成面。

## 验收

- `cargo fmt -- --check`
- `cargo test`
- `cargo clippy -- -D warnings`
- `npm --prefix server run build`
- `npm --prefix ui run build`
- CLI 命令行为保持兼容，尤其是 `watch` ready JSON。
