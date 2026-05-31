// TOML 配置读写

import fs from 'fs/promises'
import path from 'path'
import { fileURLToPath } from 'url'
import { parse, stringify } from '@iarna/toml'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const ROOT = path.resolve(__dirname, '../..')
const CONFIG_PATH = path.join(ROOT, 'omniown.toml')

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
