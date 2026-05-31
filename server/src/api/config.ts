// /api/config — 配置读取/写入（路由层 → config/index.ts 操作 TOML）

import { Router } from 'express'
import { loadConfig, saveConfig } from '../config/index.js'

export const router = Router()

// GET /api/config
router.get('/', async (_req, res) => {
  try {
    const config = await loadConfig()
    // 脱敏 api_key
    sanitizeConfig(config)
    res.json(config)
  } catch (err) {
    const msg = err instanceof Error ? err.message : '读取配置失败'
    res.status(500).json({ error: msg })
  }
})

// PUT /api/config
router.put('/', async (req, res) => {
  try {
    const newConfig = req.body

    if (!newConfig || typeof newConfig !== 'object') {
      res.status(400).json({ error: '请求体必须是 JSON 对象' })
      return
    }

    // 若 api_key 未修改（仍为脱敏值），保留原值
    const currentConfig = await loadConfig()
    if (
      currentConfig &&
      typeof currentConfig === 'object' &&
      'ai' in currentConfig &&
      typeof (currentConfig as Record<string, unknown>).ai === 'object' &&
      newConfig &&
      typeof newConfig === 'object' &&
      'ai' in newConfig &&
      typeof (newConfig as Record<string, unknown>).ai === 'object'
    ) {
      const currentAi = (currentConfig as Record<string, unknown>).ai as Record<string, unknown>
      const newAi = (newConfig as Record<string, unknown>).ai as Record<string, unknown>
      if (newAi.api_key === '***' && typeof currentAi.api_key === 'string') {
        newAi.api_key = currentAi.api_key
      }
    }

    await saveConfig(newConfig)
    res.status(204).send()
  } catch (err) {
    const msg = err instanceof Error ? err.message : '写入配置失败'
    res.status(500).json({ error: msg })
  }
})

function sanitizeConfig(config: Record<string, unknown>) {
    if (config && typeof config === 'object' && 'ai' in config) {
      const ai = (config as Record<string, unknown>).ai as Record<string, unknown>
      if (ai && typeof ai === 'object' && ai.api_key) {
        ai.api_key = '***'
      }
    }
}