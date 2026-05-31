// 请求日志中间件

import { Request, Response, NextFunction } from 'express'

export function requestLogger(req: Request, res: Response, next: NextFunction) {
  const start = Date.now()

  res.on('finish', () => {
    const duration = Date.now() - start
    const log = `${req.method} ${req.originalUrl} → ${res.statusCode} (${duration}ms)`

    if (res.statusCode >= 500) {
      console.error(log)
    } else if (res.statusCode >= 400) {
      console.warn(log)
    } else {
      console.log(log)
    }
  })

  next()
}
