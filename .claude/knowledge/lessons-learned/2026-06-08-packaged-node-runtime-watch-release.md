---
date: 2026-06-08
tags: [tauri, node, windows, packaging, watch, github-actions]
context: Windows 安装版改为自带 Node runtime 后，连续暴露 Node 启动、Release CI、Rust watch 配置路径问题
confidence: high
---

# Windows 安装版 Node runtime、watch 与 Release 排查

## 问题链

安装版从“完全不可用”逐步推进到“应用能启动但 watch 异常”。关键现象包括：

```text
Error: EISDIR: illegal operation on a directory, lstat 'D:'
[server] starting node=..., entry=\\?\D:\my_app\OmniOwn\server\dist\index.js
[watch] 配置文件解析失败 .\omniown.toml: TOML parse error
library = "C:\Users\...\com.omniown.app\library"
invalid unicode 8-digit hex code
```

GitHub Actions 也出现过两个 Release 阶段错误：

```text
Prepare bundled Node runtime: Response status code does not indicate success: 400
failed to bundle project `http status: 504`
```

## 根因

1. 安装版不能依赖用户机器预装 Node。Node 后端由 Tauri 启动，必须把 Node runtime 一起打进安装包，否则其他电脑或不同 PATH 下会失败。
2. Tauri/Windows 资源路径可能带 `\\?\` 长路径前缀。这个前缀传给 Node 作为 JS 入口参数时，Node 26 曾把 `D:` 当目录处理并触发 `EISDIR`。
3. Windows console 黑框闪现来自 `std::process::Command` 启动控制台子进程时没有设置 `CREATE_NO_WINDOW`。
4. `omniown.toml` 里直接写 `C:\Users\...` 是非法 TOML 字符串，`\U` 会被解析为 Unicode escape。Windows 路径写入 TOML 时必须转义或改用 `/`。
5. 配置文件里 `paths.library = ""` 是有效 TOML，但语义上不应传给 watch；应该当作缺省值，回退到 app data 下的 `library`。
6. PowerShell 中 `Invoke-RestMethod index.json | Where-Object ...` 如果没有强制数组枚举，可能让 `$release.version` 变成多个版本，拼出非法 Node 下载 URL。
7. Tauri MSI 打包会下载 WiX 3.14。GitHub release asset 在 Actions 中可能 504，不能只依赖 Tauri 内部一次下载。

## 已采用的修复

1. Node runtime 打包：
   - Release workflow 下载 Node 24 `win-x64.zip`。
   - 只复制 `node.exe` 和 `LICENSE` 到 `src-tauri/resources/node/win-x64/`。
   - Tauri `resources` 映射到安装目录 `node/win-x64`。
   - Rust `resolve_node_command(resource_dir)` 优先使用 bundled `node.exe`，再 fallback 到系统 Node。
2. Windows 路径：
   - 传给 Node 的 JS entry、Prisma schema、current_dir、Node command 都先去掉 `\\?\`。
   - Node 子进程设置 `CREATE_NO_WINDOW`，避免安装版黑框闪现。
3. Watch 配置：
   - Tauri 启动时调用运行时配置修复逻辑。
   - 新建或修复 `omniown.toml` 时，Windows 路径写成 `C:/Users/...`，保证 TOML 合法。
   - Rust `PathsConfig::resolve()` 把空路径字段视为默认值。
   - Node 启动 `omniown watch` 时，如果配置里的 `library` 为空，则使用 `DATABASE_URL` 同目录下的 `library` 并显式传 `--library`。
4. Release CI：
   - Node 版本解析使用 `@($releases)` 强制数组枚举，并过滤 `win-x64-zip`。
   - WiX 步骤优先复制 Windows runner 预装的 WiX 文件到 `%LOCALAPPDATA%\tauri\WixTools314`。
   - 如果没有预装 WiX，再带重试下载官方 `wix314-binaries.zip`。

## 以后排查顺序

1. 先看安装版日志：

```powershell
Get-Content "$env:APPDATA\com.omniown.app\server.log" -Tail 200
```

2. 判断进程是否真的启动：

```powershell
Get-Process node, omniown, omniown-desktop -ErrorAction SilentlyContinue
Get-NetTCPConnection -LocalPort 3001 -ErrorAction SilentlyContinue
```

3. 如果 Node 未启动，先检查：
   - `server.log` 里的 `node=` 是否指向安装目录 `node/win-x64/node.exe`
   - `entry=` 是否仍带会影响 Node 的 `\\?\`
   - 安装包内是否有 `server/dist/index.js`
4. 如果 `omniown.exe` 已启动但 watch 不对，先检查：
   - `omniown.toml` 是否有未转义反斜杠
   - `[paths] library` 是否为空
   - `[watch] 启动:` 日志是否带 `--library <绝对路径>`
   - watch 就绪日志是否监听 app data 下的 `library`，而不是 `.\library`
5. 如果 Release 失败，按 job 分层看：
   - `Prepare bundled Node runtime` 失败，多半是 Node 下载 URL 或 PowerShell JSON 枚举问题。
   - `Prepare WiX toolset` 或 `tauri-action` 下载 WiX 失败，多半是 GitHub release asset 504；优先用 runner 预装 WiX。
   - `tauri-action` 之后失败，再看 Tauri bundler、MSI 或 release upload 的具体日志。

## 验证建议

```bash
cargo check
cd src-tauri && cargo check
```

如果 WSL 内没有可用 Linux Node，不要误判为代码错误；可以直接用 Windows Node 调 TypeScript：

```cmd
pushd \\wsl$\Ubuntu\home\zj-zhuo\workspace\omniown
node server\node_modules\typescript\bin\tsc -p server\tsconfig.json
popd
```

安装版验证时，`server.log` 里理想状态应类似：

```text
[server] starting node=D:\...\OmniOwn\node\win-x64\node.exe
[watch] 启动: D:\...\OmniOwn\omniown.exe watch --db-path ... --library C:\Users\...\com.omniown.app\library
[watch] 就绪 ...
```

## 注意事项

- 不要把 `node.exe` 提交进 git；只提交 `src-tauri/resources/node/win-x64/.gitkeep` 和 `.gitignore` 规则。
- `omniown` 仍走 Tauri `externalBin`，Node runtime 走普通 `resources`。
- 修改 tag 触发 Release 前，确认 commit message 不含 `Co-authored-by`。
- 不要因为手动运行 `node dist/index.js` 缺 `DATABASE_URL` 就误判安装版失败；Tauri 正常启动时会注入该环境变量。
