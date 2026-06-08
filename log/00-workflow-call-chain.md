# OmniOwn Project Workflow And Call Chain

## Overview

OmniOwn is made of five cooperating parts:

- Tauri desktop shell: starts the desktop window, tray, runtime config repair, and backend process.
- Node API server: runs Express APIs, Prisma database setup, FTS initialization, and starts the Rust watch process.
- Rust CLI / sidecar: provides `omniown` commands such as `watch`, `process`, `extract`, and `mcp`.
- Vue UI: renders the desktop frontend and talks to the local Node API.
- SQLite / Prisma database: stores indexed documents and search metadata.

The packaged Windows app is not a single-process app. The normal runtime chain is:

```text
omniown-desktop.exe
  -> bundled node.exe
    -> server/dist/index.js
      -> omniown.exe watch
        -> SQLite database + library file watcher
```

## Release Packaging Workflow

Release packaging is driven by `.github/workflows/release.yml`.

1. A `v*` tag push or manual workflow dispatch triggers the Release workflow.
2. The `build-sidecar` job builds the Rust CLI for Windows:
   - target: `x86_64-pc-windows-msvc`
   - output: `target/x86_64-pc-windows-msvc/release/omniown.exe`
3. The `build-tauri` job downloads the sidecar artifact into `src-tauri/binaries/`.
4. The sidecar is renamed to Tauri's expected external binary name:
   - `src-tauri/binaries/omniown-x86_64-pc-windows-msvc.exe`
5. CI prepares the bundled Node runtime:
   - downloads latest Node.js `v24.*` Windows x64 zip
   - copies `node.exe` and `LICENSE`
   - target resource directory: `src-tauri/resources/node/win-x64/`
6. CI builds the Node server:
   - `npm --prefix server ci`
   - `npm --prefix server run build`
7. CI builds the Vue UI:
   - `npm --prefix ui ci`
   - `npm --prefix ui run build`
8. Tauri bundles the Windows app.
9. `src-tauri/tauri.conf.json` controls the packaged contents:
   - `externalBin`: `binaries/omniown`
   - resources:
     - `../server/dist` -> `server/dist`
     - `../server/node_modules` -> `server/node_modules`
     - `../server/prisma/schema.prisma` -> `server/dist/prisma/schema.prisma`
     - `resources/node/win-x64` -> `node/win-x64`
10. The current Windows bundle target is MSI.

## Installed App Startup Workflow

The installed app starts from `omniown-desktop.exe`.

1. Tauri enters `src-tauri/src/main.rs`.
2. During `tauri::Builder::setup()`, Tauri creates application state and tray UI.
3. Tauri calls `spawn_sidecar(app)`.
4. `spawn_sidecar()` resolves:
   - `resource_dir`: installed resource directory
   - `app_data_dir`: writable user data directory
   - bundled Node runtime: `node/win-x64/node.exe`
   - Node API entry: `server/dist/index.js`
   - Prisma schema: `server/dist/prisma/schema.prisma`
   - Rust binary: `omniown.exe`
5. Tauri verifies `node.exe --version`.
6. Tauri prepares runtime config:
   - config path: `%APPDATA%/com.omniown.app/omniown.toml`
   - default root: `%APPDATA%/com.omniown.app`
   - default library: `%APPDATA%/com.omniown.app/library`
   - the installed app directory is treated as read-only and is not used for runtime config writes
   - TOML paths are written with `/` to avoid Windows backslash escape issues.
7. Tauri starts Node with these important environment variables:
   - `DATABASE_URL=file:<app_data_dir>/dev.db`
   - `OMNIOWN_CONFIG_PATH=<app_config_dir>/omniown.toml`
   - `PRISMA_SCHEMA_PATH=<resource_dir>/server/dist/prisma/schema.prisma`
   - `OMNIOWN_BIN=<installed omniown.exe path>`
8. Tauri redirects Node stdout/stderr into:
   - `%APPDATA%/com.omniown.app/server.log`
9. Tauri monitors the Node process and retries startup a limited number of times if it exits.

## Node API Workflow

The Node API starts from `server/src/index.ts`, packaged as `server/dist/index.js`.

1. Express is initialized with CORS and JSON middleware.
2. Node resolves the Prisma schema:
   - packaged mode: `server/dist/prisma/schema.prisma`
   - development mode: `server/prisma/schema.prisma`
3. Node sets `process.env.PRISMA_SCHEMA_PATH`.
4. Node runs:
   - `npx prisma db push --skip-generate --schema="<schema>"`
5. Node attempts to set SQLite WAL mode.
6. Node initializes FTS5 via `initFts5()`.
7. Node loads runtime config through `OMNIOWN_CONFIG_PATH`.
8. Node resolves the library path:
   - prefer `[paths].library` from config when non-empty
   - fallback to `<db directory>/library`
9. Node starts Rust watch:
   - binary from `OMNIOWN_BIN` when available
   - command: `omniown.exe watch --db-path <dev.db> --library <library>`
10. Node starts Express on:
    - `http://127.0.0.1:3001`

## Rust Watch Workflow

Rust watch starts from the CLI command:

```text
omniown.exe watch --db-path <db path> --library <library path>
```

1. `src/main.rs` reads CLI arguments.
2. `bootstrap()` loads app config from `OMNIOWN_ROOT` or the current root/config directory.
3. `merge_cli_paths()` overlays CLI paths:
   - `--library` overrides `AppPaths.library`
   - `--db-path` overrides `AppPaths.db_path`
   - `DATABASE_URL=file:...` is used as a fallback database path
4. `watch::run_watch()` starts.
5. Watch initializes SQLite tables with `db::init_database(db_path)`.
6. Watch creates the library directories:
   - `<library>`
   - `<library>/public`
   - `<library>/private`
7. Watch performs an initial recursive scan of existing files.
8. Watch writes a JSON ready signal to stdout:

```json
{
  "status": "watching",
  "library": "<library path>",
  "db_path": "<db path>"
}
```

9. Node reads this stdout JSON and logs watch readiness.
10. Rust registers a recursive filesystem watcher using `notify`.
11. On file creation/change:
    - waits until the file is stable
    - indexes the file in place
    - writes document metadata/content into SQLite
12. On file deletion:
    - removes matching document records from SQLite.

## UI Call Chain

The UI is built from `ui/`.

1. In development, the UI can use a dev server.
2. In production, `ui/src/services/api-client.ts` defaults API requests to:
   - `http://127.0.0.1:3001`
3. Main API calls:
   - `GET /api/status`
   - `GET /api/documents`
   - `GET /api/documents/:id`
   - `GET /api/search?q=...`
   - `GET /api/config`
   - `PUT /api/config`
4. If Node is not running, the UI reports that it cannot reach the OmniOwn API.
5. If watch is not running but Node is running, the UI may still load API responses, but file changes will not be indexed.

## Key Runtime Paths

Example installed app directory:

```text
D:\my_app\OmniOwn
```

Important installed files:

```text
D:\my_app\OmniOwn\omniown-desktop.exe
D:\my_app\OmniOwn\omniown.exe
D:\my_app\OmniOwn\node\win-x64\node.exe
D:\my_app\OmniOwn\server\dist\index.js
D:\my_app\OmniOwn\server\dist\prisma\schema.prisma
```

The installed directory should not contain the active user configuration. It may be under
`Program Files` or another read-only location, so runtime config writes always go to AppData.

Important user data files:

```text
%APPDATA%\com.omniown.app\server.log
%APPDATA%\com.omniown.app\omniown.toml
%APPDATA%\com.omniown.app\dev.db
%APPDATA%\com.omniown.app\library
```

`omniown.toml` is the single runtime configuration file. Tauri injects this path into Node
through `OMNIOWN_CONFIG_PATH`, and Node uses it for `/api/config`, watch restarts, imports,
and AI configuration.

Important processes:

```text
omniown-desktop.exe
node.exe
omniown.exe
```

Important port:

```text
127.0.0.1:3001
```

## Troubleshooting Checkpoints

Use these checkpoints in order.

1. Confirm the installed layout contains `omniown-desktop.exe`, `omniown.exe`, bundled `node.exe`, Node server files, and Prisma schema.
2. Confirm `%APPDATA%/com.omniown.app/server.log` shows Tauri starting bundled Node.
3. Confirm the `node=` log points to the installed bundled Node runtime.
4. Confirm `entry=`, `prisma=`, and `omniown=` paths exist.
5. Confirm Node logs `OmniOwn API: http://127.0.0.1:3001`.
6. Confirm `http://127.0.0.1:3001/api/status` returns JSON.
7. Confirm `[watch] 启动:` logs an absolute `omniown.exe watch` command.
8. Confirm watch receives absolute `--db-path` and `--library` paths.
9. Confirm watch logs a ready state or `开始监听`.
10. Confirm `%APPDATA%/com.omniown.app/omniown.toml` has a non-empty TOML-safe `[paths].library`.
11. Confirm `%APPDATA%/com.omniown.app/omniown.toml` has `[paths].root` pointing to AppData unless the user intentionally set a custom root.
12. Confirm file changes under the configured library directory produce `[watch] 检测到稳定文件` and `索引完成`.

## Failure Meaning Quick Reference

- No `node.exe` process:
  - Tauri could not find or execute bundled Node, or Node exited immediately.
- Node is running but UI cannot load:
  - API port may not be listening, server startup may be failing, or UI/API base URL is wrong.
- Node API is running but no indexing:
  - Rust watch may not have started, may have exited, or may be watching the wrong library path.
- `spawn omniown ENOENT`:
  - Node could not locate the Rust sidecar binary.
- TOML parse error with Windows paths:
  - Config contains unescaped backslashes such as `C:\Users\...`; use `/` or properly escaped strings.
- Watch starts with `.\library`:
  - Runtime config library path is empty or fallback path was not applied.
