# 配置文档

> **注意：** `[embedding]` 和 `[worker]` 配置节已在 v0.1.0 移除。
> 语义搜索由 `ai-search`（LLM → 搜索词 → FTS5）替代。

## 配置加载顺序

```
1. 内置默认值（hardcoded in src/config.rs）
2. config/omniown.toml（文件配置）
3. 环境变量覆盖
```

---

## 默认配置

```toml
[paths]
root = "."
inbox = "inbox"
library = "library"
index = "index"
cache = "cache"
logs = "logs"
quarantine = "quarantine"
trash = "trash"
config_dir = "config"
database = "index/omniown.db"

[search]
default_limit = 20
fts_enabled = true

[ai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
api_key = ""
```

### 配置段说明

**`[paths]`** — 目录路径

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `root` | `"."` | 数据根目录（所有相对路径的基础） |
| `inbox` | `"inbox"` | 监控目录，支持绝对路径如 `/home/user/Downloads` |
| `library` | `"library"` | 文件存储根目录，支持绝对路径如 `/mnt/kb` |
| `index` | `"index"` | 数据库目录 |
| `database` | `"index/omniown.db"` | SQLite 数据库路径 |
| `config_dir` | `"config"` | 配置文件目录 |
| `cache` / `logs` / `quarantine` / `trash` | — | 运行时目录 |

> `inbox` 和 `library` 支持绝对路径。若为相对路径则拼接 `root`，若为绝对路径则原样使用。

**`[search]`** — 搜索配置

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `default_limit` | `20` | 默认搜索结果条数 |
| `fts_enabled` | `true` | 启用 FTS5 全文搜索 |

**`[ai]`** — AI 搜索配置

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `base_url` | `"https://api.openai.com/v1"` | LLM API 地址 |
| `model` | `"gpt-4o-mini"` | 模型名 |
| `api_key` | `""` | API 密钥 |

---

## 环境变量

| 变量 | 覆盖 | 示例 |
|------|------|------|
| `OMNIOWN_ROOT` | `paths.root` | `OMNIOWN_ROOT=/data/notes` |
| `OMNIOWN_DB_PATH` | `paths.database` | `OMNIOWN_DB_PATH=/custom/db.sqlite` |

**注意：**

- 环境变量均为可选的
- `OMNIOWN_ROOT` 影响所有相对路径的解析
- 其他配置由 `config/omniown.toml` 或 Tauri 桌面设置界面管理
