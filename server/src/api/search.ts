// /api/search — FTS5 全文搜索 + AI 多策略搜索

import { Router } from 'express'
import { searchDocuments } from '../services/search.service.js'
import { aiSearchWithTrace } from '../services/ai.service.js'

export const router = Router()

// GET /api/search?q=关键词            → FTS5 全文搜索
// GET /api/search?q=关键词&ai=true    → AI 智能搜索（多策略）

router.get('/', async (req, res) => {
  try {
    const query = req.query.q
    if (typeof query !== 'string' || query.trim().length === 0) {
      res.status(400).json({ error: '缺少搜索词，请提供 q 参数' })
      return
    }

    // 如果请求 ai 模式，走 AI 多策略搜索
    if (req.query.ai === 'true') {
      const { results, trace } = await aiSearchWithTrace(query.trim())
      res.json({ results, trace })
      return
    }

    // 默认走 FTS5 全文搜索
    const results = await searchDocuments(query.trim())
    res.json({ results })
  } catch (err) {
    const msg = err instanceof Error ? err.message : '搜索失败'
    res.status(500).json({ error: msg, trace: { error: msg } })
  }
})
