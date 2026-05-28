// ============================================================
// 请求日志中间件（Morgan）
// ============================================================
//
// 记录每个 HTTP 请求的方法、路径、状态码、耗时。
//
// 生产环境推荐使用 morgan 包：
//   npm install morgan
//   npm install -D @types/morgan
//
//   import morgan from 'morgan'
//   app.use(morgan('combined'))  // Apache 格式日志
//
// 下面是一个简易的手写版本，用于理解中间件原理。
//
// ============================================================

import { Request, Response, NextFunction } from 'express'

// 简易请求日志中间件
export function requestLogger(req: Request, res: Response, next: NextFunction) {
  // --- 记录开始时间 ---
  // Date.now() 返回自 1970-01-01 以来的毫秒数
  const start = Date.now()

  // --- 监听响应结束事件 ---
  // res 是一个 EventEmitter，会触发 finish 事件
  res.on('finish', () => {
    // 计算耗时
    const duration = Date.now() - start

    // 拼接日志信息
    // 模板字符串（反引号）中可以用 ${} 嵌入表达式
    const log = `${req.method} ${req.originalUrl} → ${res.statusCode} (${duration}ms)`

    // 4xx 用 console.warn，5xx 用 console.error，其他用 console.log
    if (res.statusCode >= 500) {
      console.error(log)
    } else if (res.statusCode >= 400) {
      console.warn(log)
    } else {
      console.log(log)
    }
  })

  // --- 调用下一个中间件 ---
  // 如果不调用 next()，请求就在这里停下了
  next()
}

// ============================================================
// 理解中间件流程
// ============================================================
//
// 请求进入 Express 时的顺序：
//
//   req
//   │
//   ▼
//   cors()                       ← 跨域处理
//   │
//   ▼
//   express.json()               ← 解析 JSON 请求体
//   │
//   ▼
//   requestLogger                ← 记录日志（你在这里）
//   │
//   ▼
//   router.get('/api/documents') ← 路由匹配
//   │
//   ▼
//   handler                      ← 你的业务代码
//   │
//   ▼
//   res.json()                   ← 返回响应
//
// 中间件的核心是 next()：每个中间件做完自己的事后，
// 调用 next() 把控制权交给下一个。
// 如果不调用 next()，请求就卡住了。
