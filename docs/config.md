# 配置文档

## 配置加载顺序

```
1. 内置默认值（代码中硬编码）
2. config/omniown.toml（TOML 配置文件）
3. 环境变量覆盖（OMNIOWN_ROOT, OMNIOWN_DB_PATH）
```

Node.js 后端通过 `server/src/config/index.ts` 中的 `loadConfig()` 读取 TOML 文件。

---

## 配置文件 (config/omniown.toml)

```toml
[paths]
root = "."
inbox = "inbox"
library = "library"
database = "index/omniown.db"

[search]
default_limit = 20
fts_enabled = true

[ai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
api_key = ""
```

### `[paths]` — 目录路径

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `root` | `"."` | 数据根目录 |
| `inbox` | `"inbox"` | 待导入文件目录，支持绝对路径 |
| `library` | `"library"` | 文件存储目录，支持绝对路径 |
| `database` | `"index/omniown.db"` | SQLite 数据库文件路径 |

### `[search]` — 搜索配置

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `default_limit` | `20` | 搜索结果条数 |
| `fts_enabled` | `true` | FTS5 全文搜索开关 |

### `[ai]` — AI 搜索配置

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `base_url` | `"https://api.openai.com/v1"` | LLM API 地址（支持 OpenAI/Ollama 等兼容接口） |
| `model` | `"gpt-4o-mini"` | 模型名 |
| `api_key` | `""` | API 密钥（必填，Ollama 本地服务可留空） |

---

## API 路由中的配置

| 方法 | 路径 | 说明 |
|:---|:-----|:----|
| GET | `/api/config` | 读取配置（api_key 脱敏：`sk-a***`） |
| PUT | `/api/config` | 更新配置（校验字段类型和格式） |

PUT 请求会校验：
- `ai.base_url` 必须以 `http://` 或 `https://` 开头
- `search.default_limit` 必须是正整数
- `search.fts_enabled` 必须是布尔值

---

## 环境变量

| 变量 | 覆盖 | 示例 |
|------|------|------|
| `OMNIOWN_ROOT` | `paths.root` | `OMNIOWN_ROOT=/data/notes` |
| `DATABASE_URL` | Prisma 连接 | `DATABASE_URL="file:./dev.db"` |
