# 配置与环境变量

## 配置文件位置

| 场景 | 路径 | 说明 |
|------|------|------|
| Tauri 打包 | `{app_config_dir}/omniown.toml` | Windows: `%APPDATA%/com.omniown.app/` |
| 开发模式 | 项目根目录 `/omniown.toml` | 通过 `OMNIOWN_CONFIG_PATH` env 可覆盖 |
| 兜底 | Node.js `path.resolve(__dirname, '../../..')` | 仅当无 env var 时 |

## 环境变量

| 变量 | 用途 | 设置方 | 必需 |
|------|------|------|------|
| `DATABASE_URL` | SQLite 数据库路径（`file:...`） | Tauri / .env | 是 |
| `OMNIOWN_CONFIG_PATH` | TOML 配置文件绝对路径 | Tauri（启动时注入） | 打包时是 |
| `OMNIOWN_ROOT` | 数据根目录 | 用户 | 否 |
| `OMNIOWN_DB_PATH` | 数据库文件路径 | 用户 | 否 |

## 配置节格式 (omniown.toml)

```toml
[paths]
root = "."
library = "library"

[search]
default_limit = 20
fts_enabled = true

[ai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
api_key = ""
```

## 配置读流程

```
Tauri: read_config/write_config → app_config_dir/omniown.toml
Node.js: loadConfig() → OMNIOWN_CONFIG_PATH env 优先 → 推算路径 → 返回 {} 兜底
Rust CLI: bootstrap() → OMNIOWN_ROOT env → ./omniown.toml → ./config/omniown.toml → 默认值
```

## 配置写流程

1. 前端 ConfigView 调用 `PUT /api/config`
2. Node.js `api/config.ts` 写入 TOML 文件
3. **同时**通过 Tauri IPC `write_config` 写入 `app_config_dir`（双写同步）
4. 触发 Node.js 进程重启以应用新路径配置
