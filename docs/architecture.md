# 架构文档

## 总体架构

```
Tauri 桌面壳 (src-tauri/)
  ├── 系统托盘 + 悬浮面板
  ├── 启动时 spawn Node.js sidecar
  └── WebView 渲染 Vue 前端
         │
    ┌────┘
    ▼
Vue 3 + TS (ui/)  ─── HTTP ─── Node.js/TS API (server/)
     前端                         后端
                                    ├── Express 路由层
                                    ├── Prisma ORM + SQLite
                                    ├── LLM 智能搜索
                                    └── Rust CLI 集成
                                          │
                                          │ child_process
                                          │
                                          └── Rust CLI (src/)
                                               ├── extractor (文本提取)
                                               ├── processor (文件管线)
                                               └── mcp (MCP Server)
```

## 技术栈

| 层 | 技术 |
|:---|------|
| 桌面壳 | Tauri v1 (WebView + system-tray) |
| 前端 | Vue 3 + TypeScript + Vite |
| 后端 | Node.js + Express + TypeScript |
| 数据库 | SQLite + Prisma ORM + FTS5 |
| AI 搜索 | LLM → 策略选择 → FTS5 |
| 核心处理 | Rust CLI (child_process) |

## 数据流

```
用户打开桌面应用
  ↓
Tauri 壳启动 → spawn Node.js API (port 3001)
  ↓
WebView 加载 Vue 前端 → HTTP API 调用
  ↓
Prisma → SQLite (FTS5 + documents 表)
  ↓ (文件导入时)
Node.js execa → Rust CLI ("omniown process <file>")
  ↓
extractor → classifier → storage → db::upsert
```

## API 路由

| 方法 | 路径 | 说明 |
|:---|------|:-----|
| GET | `/api/status` | 系统状态（文档统计） |
| GET | `/api/documents` | 文档列表（不含 content） |
| GET | `/api/documents/:id` | 文档详情（含 content） |
| GET | `/api/search?q=` | FTS5 全文搜索 |
| GET | `/api/config` | 读取配置 |
| PUT | `/api/config` | 更新配置 |

## 搜索架构

```
用户输入 "我上周的代码文件"
      ↓
AI 搜索 (ai.search.ts)
      ↓
LLM → [{ strategy: "recent", params: { days: "7" } },
        { strategy: "category", params: { keyword: "code" } }]
      ↓
并行执行策略 (search.service.ts)
      ↓
合并去重 → top 20 → 返回
```

8 个搜索策略：`fulltext` / `category` / `filetype` / `summary` / `recent` / `privacy` / `filename` / `tag`

## 目录结构

```
omniown/
├── server/               # Node.js/TS API
│   ├── src/
│   │   ├── index.ts             # Express 入口
│   │   ├── api/                 # 路由层
│   │   ├── services/            # 业务逻辑层
│   │   ├── db/                  # 数据库
│   │   ├── config/              # 配置管理
│   │   └── middleware/          # 中间件
│   └── prisma/
├── ui/                   # Vue 3 前端
├── src/                  # Rust 核心（仅保留 CLI）
├── src-tauri/            # Tauri 桌面壳
└── docs/                 # 文档
```
