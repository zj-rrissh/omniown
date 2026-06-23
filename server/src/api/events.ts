// GET /api/events — SSE 事件流

import { Router } from 'express'
import { addSseClient } from '../services/events.service.js'

export const router = Router()

router.get('/', (req, res) => {
  // SSE 要求
  res.setHeader('Content-Type', 'text/event-stream')
  res.setHeader('Cache-Control', 'no-cache')
  res.setHeader('Connection', 'keep-alive')
  res.setHeader('X-Accel-Buffering', 'no')

  // 允许跨域（开发时 Vite 代理到不同端口）
  res.setHeader('Access-Control-Allow-Origin', '*')

  const cleanup = addSseClient(res)

  // 如果客户端提前关闭，清理连接
  req.on('close', cleanup)
})
