// /api/status — 数据库统计

import { Router } from 'express'
import prisma from '../db/client.js'
import { loadConfig, resolveConfigPaths } from '../config/index.js'
import { resolveDbPath } from '../utils/omniown-cli.js'

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
    const config = await loadConfig()
    const resolvedPaths = resolveConfigPaths(config)
    const dbPath = resolveDbPath() || resolvedPaths.database || ''

    res.json({
      database: dbPath,
      root: resolvedPaths.root,
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
