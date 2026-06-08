// TOML 配置读写

import fs from 'fs/promises'
import path from 'path'
import { fileURLToPath } from 'url'
import { parse, stringify } from '@iarna/toml'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// 优先读取 Tauri 注入的环境变量（OS 用户数据目录），
// 否则回退到基于 __dirname 推算的 exe/项目根目录
export const CONFIG_PATH = process.env.OMNIOWN_CONFIG_PATH
  || path.join(path.resolve(__dirname, '../../..'), 'omniown.toml')

console.log('[config] path:', CONFIG_PATH)

export async function loadConfig(): Promise<Record<string, unknown>> {
  try {
    const content = await fs.readFile(CONFIG_PATH, 'utf-8')
    return parse(content) as Record<string, unknown>
  } catch {
    return {}
  }
}

export async function saveConfig(config: unknown): Promise<void> {
  await fs.mkdir(path.dirname(CONFIG_PATH), { recursive: true })
  await fs.writeFile(CONFIG_PATH, stringify(config as any), 'utf-8')
}
