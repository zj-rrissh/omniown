// /api/documents — 文档列表与详情

import { Router } from 'express'
import prisma from '../db/client.js'

export const router = Router()

// GET /api/documents
router.get('/', async (req, res) => {
  try {
    const docs = await prisma.document.findMany({
      select: {
        id: true,
        filename: true,
        storedPath: true,
        fileExt: true,
        fileSize: true,
        folderType: true,
        category: true,
        domain: true,
        docType: true,
        riskLevel: true,
        processingStatus: true,
        createdAt: true,
        updatedAt: true,
      },
      orderBy: { updatedAt: 'desc' },
      take: 20,
    })
    res.json({ documents: docs })
  } catch (err) {
    const msg = err instanceof Error ? err.message : '查询失败'
    res.status(500).json({ error: msg })
  }
})

// GET /api/documents/:id
router.get('/:id', async (req, res) => {
  try {
    const id = Number(req.params.id)

    if (isNaN(id)) {
      res.status(400).json({ error: '无效的文档 ID' })
      return
    }

    const doc = await prisma.document.findUnique({ where: { id } })

    if (!doc) {
      res.status(404).json({ error: '文档不存在' })
      return
    }

    res.json({ document: doc })
  } catch (err) {
    const msg = err instanceof Error ? err.message : '查询失败'
    res.status(500).json({ error: msg })
  }
})
