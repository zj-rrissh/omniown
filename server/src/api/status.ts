// ============================================================
// /api/status — 系统状态
// ============================================================
//
// 返回数据库统计信息：文档总数、分类分布、Schema 版本等。
//
// ============================================================

import { Router } from 'express'
import prisma from '../db/client.js'

export const router = Router()

router.get('/status', async (_req, res) => {
  try {
    // --- 并发查询所有统计 ---
    //
    // Promise.all 并行执行多个异步查询，减少等待时间。
    // 所有 Prisma 查询共享同一个 prisma 单例连接。
    const [
      total,
      publicCount,
      privateCount,
      indexedCount,
      failedCount,
    ] = await Promise.all([
      // 文档总数
      prisma.document.count(),

      // 公开文档数
      prisma.document.count({ where: { folderType: 'public' } }),

      // 私有文档数
      prisma.document.count({ where: { folderType: 'private' } }),

      // 已索引文档数
      prisma.document.count({
        where: { processingStatus: 'indexed' },
      }),

      // 处理失败文档数
      prisma.document.count({
        where: { processingStatus: 'failed' },
      }),
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
