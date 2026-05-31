// /api/status — 数据库统计

import { Router } from 'express'
import prisma from '../db/client.js'

export const router = Router()

router.get('/status', async (_req, res) => {
  try {
    const [
      total,
      publicCount,
      privateCount,
      indexedCount,
      failedCount,
    ] = await Promise.all([
      prisma.document.count(),
      prisma.document.count({ where: { folderType: 'public' } }),
      prisma.document.count({ where: { folderType: 'private' } }),
      prisma.document.count({ where: { processingStatus: 'indexed' } }),
      prisma.document.count({ where: { processingStatus: 'failed' } }),
    ])

    res.json({
      database: 'omniown.db',
      root: 'data',
      schema: {
        current_version: 5,
        pending_migrations: 0,
      },
      documents: {
        total,
        public: publicCount,
        private: privateCount,
        indexed: indexedCount,
        failed: failedCount,
      },
    })
  } catch (err) {
    const msg = err instanceof Error ? err.message : '状态查询失败'
    res.status(500).json({ error: msg })
  }
})
