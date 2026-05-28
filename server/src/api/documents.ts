// ============================================================
// /api/documents 路由模块
// ============================================================
//
// Express Router 把一组相关路由从入口文件拆分到独立文件。
// router 可以看作一个"迷你 app"，有自己的路由和中间件。
// 最后在 index.ts 中用 app.use('/api/documents', router) 挂载。
//
// ============================================================

import { Router } from 'express'
import prisma from '../db/client.js'

// Router() 创建一个路由实例。
// 它的用法和 app.get() / app.post() 完全一样。
export const router = Router()

// ------- GET /api/documents — 文档列表 -------

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
    res.json(docs)
  } catch (err) {
    // 数据库查询出错时返回 500
    // err 是 unknown 类型，需要先判断再取 message
    const msg = err instanceof Error ? err.message : '查询失败'
    res.status(500).json({ error: msg })
  }
  // 注意：不再需要 prisma.$disconnect()
  // 因为 prisma 是单例，由应用进程统一管理连接生命周期
})

// ------- GET /api/documents/:id — 文档详情 -------

// :id 是路由参数（URL parameter）
// 当访问 /api/documents/5 时，req.params.id === "5"
router.get('/:id', async (req, res) => {
  try {
    // req.params 中的所有值都是 string 类型！
    // 但数据库的 id 是 Int，所以需要用 Number() 转换
    const id = Number(req.params.id)

    // 如果传入的参数不是有效数字（如 "abc"），Number() 返回 NaN
    // isNaN() 检查是否为 NaN
    if (isNaN(id)) {
      // 400 Bad Request — 客户端请求格式错误
      res.status(400).json({ error: '无效的文档 ID' })
      return
      // 注意：Express 中 res.json() 不会中断函数执行
      // 必须写 return 或 else，否则代码会继续往下跑
    }

    // findUnique: 通过主键或唯一字段查询单条记录
    // where: 查询条件，{ id: id } 表示 WHERE id = ?
    const doc = await prisma.document.findUnique({
      where: { id },
      // 详情接口返回完整字段（包括 content）
      // 省略了部分管理字段
    })

    // findUnique 查不到时返回 null，不是抛异常
    if (!doc) {
      // 404 Not Found — 资源不存在
      res.status(404).json({ error: '文档不存在' })
      return
    }

    res.json(doc)
  } catch (err) {
    const msg = err instanceof Error ? err.message : '查询失败'
    res.status(500).json({ error: msg })
  }
})
