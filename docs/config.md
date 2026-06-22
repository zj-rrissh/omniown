# 配置文档

## 配置文件位置

| 运行模式 | 路径 |
|:---|:---|
| Tauri 桌面端 | `{app_config_dir}/omniown.toml`（平台用户配置目录） |
| Node.js 独立运行 | `<server_root>/omniown.toml` |

## 配置格式 (omniown.toml)

```toml
[ai]
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
api_key = "sk-..."

[paths]
root = "."
library = "library"
```

### `[ai]` — AI 搜索配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `base_url` | string | `"https://api.deepseek.com"` | LLM API 地址（兼容 OpenAI 接口规范） |
| `model` | string | `"deepseek-v4-flash"` | 模型名 |
| `api_key` | string | `""` | API 密钥（Ollama 本地服务可留空） |

### `[paths]` — 存储路径

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `root` | string | `""` | 数据根目录，作为相对路径的基准 |
| `library` | string | `""` | 知识库目录，文件放入即自动索引，支持绝对路径和相对路径 |

留空时使用默认值（相对于数据根目录）。

---

## 配置 API

| 方法 | 路径 | 说明 |
|:---|:---|:---|
| GET | `/api/config` | 读取配置，`api_key` 脱敏为 `***` |
| PUT | `/api/config` | 更新配置，`api_key` 为 `***` 时保留原值 |

### GET 响应格式

```json
{
  "ai": { "base_url": "https://api.deepseek.com", "model": "deepseek-v4-flash", "api_key": "***" },
  "paths": { "root": ".", "library": "library" }
}
```

### PUT 请求格式

```json
{
  "ai": { "base_url": "https://api.deepseek.com", "model": "deepseek-v4-flash", "api_key": "***" },
  "paths": { "root": ".", "library": "library" }
}
```

改变配置后 Tauri 端会 kill 并自动重启 sidecar 进程使新配置生效。

---

## 双层配置说明

项目存在两个配置消费端：

| 端 | 消费方式 | 说明 |
|:---|:---|:---|
| Node.js API | `server/src/config/index.ts` → `loadConfig()` | 读取 TOML 提供 AI 配置给搜索服务 |
| Tauri (Rust) | `src-tauri/src/main.rs` → `read_config()` / `read_paths_config()` | 读取 TOML 提供路径给前端展示 |

两者读取**同一个 TOML 文件**，通过 `write_config` 命令统一写入。

---

## 环境变量

| 变量 | 用途 | 示例 |
|------|------|------|
| `DATABASE_URL` | Prisma 数据库连接，启动时由 Tauri 壳注入 | `DATABASE_URL="file:/path/to/omniown.db"` |

> 注意：当前配置**没有**环境变量覆盖机制。所有配置通过 TOML 文件管理，由 Tauri 壳注入 `DATABASE_URL`。
