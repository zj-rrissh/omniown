// ============================================================
// 错误处理中间件
// ============================================================
//
// Express 的错误处理中间件有 4 个参数：(err, req, res, next)
// 普通中间件有 3 个参数：(req, res, next)
// Express 根据参数个数来判断是错误处理中间件还是普通中间件。
//
// 错误处理中间件必须写满 4 个参数，即使不用也要留着。
//
// ============================================================

import { Request, Response, NextFunction } from 'express'

// --- 404 处理 — 匹配不到任何路由时触发 ---
//
// 为什么 404 也是一个"错误"？
// 在 Express 中，如果所有路由都没匹配到请求，
// 中间件链就结束了，客户端会一直等响应。
// 所以需要兜底中间件来返回 404。
//
// 404 处理是一个普通中间件（3 个参数），
// 不是错误处理中间件（4 个参数）。
// 它应该放在所有路由之后注册。
export function notFoundHandler(_req: Request, res: Response, _next: NextFunction) {
  // 设置 HTTP 状态码为 404
  // 默认是 200，需要显式设置
  res.status(404).json({
    error: 'Not Found',
    // 可以加上请求的路径帮助调试
    // path: req.originalUrl,
  })
}

// --- 全局错误处理中间件 ---
//
// 当路由中调用 next(err) 或抛出异常时，
// Express 会跳过所有普通中间件，直接进入错误处理中间件。
//
// 参数解释：
// err   — 错误对象（可能是 Error 实例，也可能是任何值）
// req   — 请求对象
// res   — 响应对象
// next  — 下一个中间件（很少用，通常用于传递给默认错误处理器）
//
// 注意：即使没用 req，也必须声明 4 个参数，
// 否则 Express 不认为这是一个错误处理中间件。
export function errorHandler(
  err: unknown,
  _req: Request,
  res: Response,
  _next: NextFunction
) {
  // --- 确定状态码 ---
  //
  // 约定：如果 err 对象上有 status 或 statusCode 属性，就用它
  // 否则默认 500（服务器内部错误）
  const statusCode =
    (err as Record<string, unknown>)?.status as number
    ?? (err as Record<string, unknown>)?.statusCode as number
    ?? 500

  // --- 确定错误消息 ---
  //
  // 不同来源的错误有不同的消息提取方式：
  const message =
    // 1. 自定义错误消息（优先用 status + message 对象）
    (err as Record<string, unknown>)?.message as string
    // 2. Error 实例
    ?? (err instanceof Error ? err.message : undefined)
    // 3. 兜底
    ?? 'Internal Server Error'

  // --- 记录日志 ---
  //
  // 5xx 错误记录完整错误栈，便于调试
  if (statusCode >= 500) {
    console.error('[ERROR]', err instanceof Error ? err.stack : err)
  } else {
    // 4xx 错误是客户端的问题，不需要打印堆栈
    console.warn('[WARN]', statusCode, message)
  }

  // --- 返回错误响应 ---
  //
  // 生产环境不要暴露详细的 error stack 给客户端
  res.status(statusCode).json({
    error: message,
    // 可选：添加请求 ID，方便日志关联
    // requestId: req.id,
  })
}

// ============================================================
// 使用方式
// ============================================================
//
// 在 index.ts 中按以下顺序注册：
//
//   import { notFoundHandler, errorHandler } from './middleware/error.js'
//
//   1. 路由（app.get / app.post / router.get）
//   2. 404 兜底（notFoundHandler）
//   3. 错误处理（errorHandler）
//
//   app.get('/api/xxx', handler)
//   app.use(notFoundHandler)     // ← 兜底：所有未匹配的路由
//   app.use(errorHandler)        // ← 兜底：所有错误
//
// ============================================================
//
// 延伸：自定义错误类
//
// 如果你想让错误处理更规范，可以定义一个 AppError 类：
//
//   export class AppError extends Error {
//     constructor(
//       public statusCode: number,
//       message: string
//     ) {
//       super(message)
//       this.name = 'AppError'
//     }
//   }
//
// 路由中这样用：
//   throw new AppError(403, '没有权限')
//   throw new AppError(404, '文档不存在')
//
// 然后在 errorHandler 中直接读取 statusCode 属性。
