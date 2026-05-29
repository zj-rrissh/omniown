// ============================================================
// /api/config — 配置读取/写入
// ============================================================
//
// 路由层：只负责 HTTP 请求/响应，不直接操作文件系统。
// 具体的配置文件读写调用 config/index.ts 中的方法。
//
// 职责分离：
//   config.ts（路由层）     ← HTTP + JSON 序列化
//       ↓ 调用
//   config/index.ts（服务层） ← 文件系统 + TOML 解析
//
// ============================================================

import { Router } from 'express'
import { loadConfig, saveConfig } from '../config/index.js'

export const router = Router()

// ------- GET /api/config — 读取配置 -------

router.get('/', async (_req, res) => {
  try {
    // 读取 TOML 配置文件，返回 JavaScript 对象
    const config = await loadConfig()

    // 脱敏处理：不把完整 api_key 返回给前端
    // 注意这里的 config 类型是 Record<string, unknown>
    // 需要用类型断言或 in 操作符来判断字段是否存在
    sanitizeConfig(config)

    res.json(config)
  } catch (err) {
    const msg = err instanceof Error ? err.message : '读取配置失败'
    res.status(500).json({ error: msg })
  }
})

// ------- PUT /api/config — 更新配置 -------

router.put('/', async (req, res) => {
  try {
    // req.body 是客户端传来的新配置（Express 已解析为 JavaScript 对象）
    const newConfig = req.body

    // 你也可以在这里加字段校验：
    if (!newConfig || typeof newConfig !== 'object') {
      res.status(400).json({ error: '请求体必须是 JSON 对象' })
      return
    }

    // 写入 TOML 文件
    await saveConfig(newConfig)

    // 204 No Content：成功但无返回体
    res.status(204).send()
  } catch (err) {
    const msg = err instanceof Error ? err.message : '写入配置失败'
    res.status(500).json({ error: msg })
  }
})

// ============================================================
// 练习
// ============================================================
//
// 1. 把脱敏逻辑抽成一个单独的函数 sanitizeConfig(config)，复用
// 2. 给 PUT 加校验：确保 newConfig 是对象、必填字段存在
// 3. 把 @iarna/toml 换成轻量的 smol-toml（替代品）

function sanitizeConfig(config: Record<string, unknown>) {
    if (config && typeof config === 'object' && 'ai' in config) {
      const ai = (config as Record<string, unknown>).ai as Record<string, unknown>
      if (ai && typeof ai === 'object' && ai.api_key) {
        ai.api_key = '***'
      }
    }
}