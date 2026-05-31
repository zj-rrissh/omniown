// 全局错误处理中间件

import { Request, Response, NextFunction } from 'express'

// 404 兜底 — 所有路由之后注册
export function notFoundHandler(_req: Request, res: Response, _next: NextFunction) {
  res.status(404).json({ error: 'Not Found' })
}

// 全局错误处理 — Express 4 参数中间件
export function errorHandler(
  err: unknown,
  _req: Request,
  res: Response,
  _next: NextFunction
) {
  const statusCode =
    (err as Record<string, unknown>)?.status as number
    ?? (err as Record<string, unknown>)?.statusCode as number
    ?? 500

  const message =
    (err as Record<string, unknown>)?.message as string
    ?? (err instanceof Error ? err.message : undefined)
    ?? 'Internal Server Error'

  if (statusCode >= 500) {
    console.error('[ERROR]', err instanceof Error ? err.stack : err)
  } else {
    console.warn('[WARN]', statusCode, message)
  }

  res.status(statusCode).json({ error: message })
}
