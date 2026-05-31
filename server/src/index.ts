// OmniOwn API 入口 — 路由挂载与服务器启动

import express from 'express'
import cors from 'cors'

const app = express()

// --- 全局中间件 ---
app.use(cors())
app.use(express.json())

// --- 数据库初始化 ---
// FTS5 虚拟表需要手动创建（Prisma 不支持）
import { initFts5 } from './db/setup-fts.js'
await initFts5()

// --- 路由挂载 ---

import { router as statusRouter } from './api/status.js'
import { router as documentsRouter } from './api/documents.js'
import { router as configRouter } from './api/config.js'
import { router as searchRouter } from './api/search.js'

app.use('/api', statusRouter)          // GET  /api/status
app.use('/api/documents', documentsRouter)  // GET  /api/documents
                                       // GET  /api/documents/:id
app.use('/api/config', configRouter)   // GET  /api/config
                                       // PUT  /api/config
app.use('/api/search', searchRouter)   // GET  /api/search?q=

// --- 启动服务器 ---

app.listen(3001, () => {
  console.log('OmniOwn API: http://127.0.0.1:3001')
})
