// ============================================================
// OmniOwn 服务端入口
// ============================================================
//
// 这个文件负责：
// 1. 创建 Express 应用
// 2. 注册全局中间件（cors、json 解析）
// 3. 挂载路由模块
// 4. 启动 HTTP 服务器
//
// 路由的具体实现拆分到了 src/api/ 下的各个模块文件中。
// 这样每个 API 文件职责单一，易于维护和测试。
//
// ============================================================

import express from 'express'
import cors from 'cors'

const app = express()

// --- 全局中间件 ---
app.use(cors())
app.use(express.json())

// --- 路由挂载 ---
//
// 路由模块用 export const router = Router() 导出，
// 在这里用 app.use(path, router) 挂载。
//
// 挂载时指定的路径（如 /api/documents）会作为前缀：
// router.get('/')        → GET /api/documents
// router.get('/:id')     → GET /api/documents/:id

// TODO: 取消注释以下两行来挂载 documents 路由
import { router as documentsRouter } from './api/documents.js'
app.use('/api/documents', documentsRouter)

// --- 内联路由（待拆分） ---

// GET /api/status — 系统状态
// 这个路由暂时留在 index.ts，后续也可以拆分出去
app.get('/api/status', (_req, res) => {
  res.json({
    database: 'omniown.db',
    root: 'data',
    schema: { current_version: 5, pending_migrations: 0 },
    documents: { total: 0, public: 0, private: 0, indexed: 0, failed: 0 },
  })
})

// --- 启动服务器 ---

app.listen(3001, () => {
  console.log('OmniOwn API: http://127.0.0.1:3001')
})
