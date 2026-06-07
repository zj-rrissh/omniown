---
date: 2026-06-07
tags: [tauri, node, sidecar, prisma, packaging]
context: Tauri 安装版中 Node API 已启动，但内部调用 Rust CLI 失败，并伴随 Prisma schema 路径风险
confidence: high
---

# 打包后 sidecar 与 Prisma 路径排查

## 问题

VSCode 中分别启动前后端可以正常运行，但安装版无法使用。关键日志包括：

```text
OmniOwn API: http://127.0.0.1:3001
[watch] 启动失败: spawn omniown ENOENT
Could not load prisma/schema.prisma
```

这说明问题不是 Node 后端完全没有启动，而是 Node 后端启动后，内部依赖的 Rust CLI 子进程或 Prisma schema 路径在安装环境中失效。

## 判断链

1. `OmniOwn API` 已输出，说明 `server/dist/index.js` 至少能启动 HTTP 服务。
2. `spawn omniown ENOENT` 表示 Node 内部执行了 `spawn("omniown")` 或同等调用，但安装环境的 `PATH` 中找不到该可执行文件。
3. 开发环境能找到 CLI，通常是因为 `target/debug`、`target/release` 或本机 `PATH` 恰好可用；安装版不会继承这些路径假设。
4. Prisma 的 `DATABASE_URL` 在 Tauri 中由父进程注入；手动运行 Node 时缺失 `DATABASE_URL` 属于预期现象，不应误判为安装版根因。
5. `schema.prisma` 需要使用打包后的绝对路径，避免 Prisma 仍按默认 `prisma/schema.prisma` 解析。

## 解决方案

1. 在 Node 后端抽出公共 CLI helper，统一解析 `omniown` 二进制：
   - 优先使用 `process.env.OMNIOWN_BIN`
   - 再查找开发环境 `target/debug` 与 `target/release`
   - 再查找打包资源目录中的 `omniown.exe` 或 `binaries/omniown-<target-triple>`
   - 最后才回退到 `omniown`
2. Tauri 启动 Node 后端时注入：
   - `DATABASE_URL`
   - `OMNIOWN_CONFIG_PATH`
   - `OMNIOWN_BIN`
   - `PRISMA_SCHEMA_PATH`
3. `watch` 和导入服务都必须使用同一套 CLI helper，避免一个路径修好了、另一个接口仍然 `exec("omniown ...")`。
4. 导入服务应使用参数化 `spawn`，不要拼接 shell 命令字符串，避免 Windows 路径空格和文件名引号问题。
5. `process` 和 `watch` 都要把同一个 `--db-path` 传给 Rust CLI，避免写入另一个默认数据库造成 split-brain。

## 打包注意事项

- Windows 安装版如果依赖 Tauri `externalBin: ["binaries/omniown"]`，需要准备 `src-tauri/binaries/omniown-x86_64-pc-windows-msvc.exe`。
- 如果安装目录同级已经包含 `omniown.exe`，Tauri 也应显式把该绝对路径注入给 Node，不能指望 Node 自动从安装目录定位。
- Tauri 子进程重启逻辑必须复制初次启动的全部 env 和 `current_dir`，否则首次启动正常、重启后失效。
- `server/dist/prisma/schema.prisma` 必须存在于资源目录；仅复制 `server/prisma/schema.prisma` 到错误位置会让 Prisma 默认路径解析失败。
- UNC 路径下 Windows `npm` 和 `cargo` 可能出现非代码错误；验证时可切到 WSL 路径或关闭 incremental。

## 验证建议

```bash
node server/node_modules/typescript/bin/tsc -p server/tsconfig.json
CARGO_INCREMENTAL=0 cargo check
```

还可以直接加载编译后的 helper 检查开发环境解析结果：

```bash
node --input-type=module -e "import('./server/dist/utils/omniown-cli.js').then(m=>console.log(m.resolveOmniownBinary()))"
```

预期开发环境应解析到 `target/debug/omniown` 或 `target/release/omniown`；安装版应解析到 Tauri 注入的 `OMNIOWN_BIN`。
