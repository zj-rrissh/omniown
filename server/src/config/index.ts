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

export interface ResolvedConfigPaths {
  root: string
  library: string
  database?: string
  runtime_base: string
  config_path: string
}

export async function loadConfig(): Promise<Record<string, unknown>> {
  try {
    const content = await fs.readFile(CONFIG_PATH, 'utf-8')
    return parse(content) as Record<string, unknown>
  } catch {
    return {}
  }
}

export async function saveConfig(config: unknown): Promise<void> {
  const dir = path.dirname(CONFIG_PATH)
  await fs.mkdir(dir, { recursive: true })

  // 读取现有配置，与新配置深度合并，避免丢失 prompt_variant 等前端未发送的字段
  let existing: Record<string, unknown> = {}
  try {
    const content = await fs.readFile(CONFIG_PATH, 'utf-8')
    existing = parse(content) as Record<string, unknown>
  } catch {
    // 文件不存在或不可读，以新配置为准
  }

  const merged = deepMerge(existing, config as Record<string, unknown>)
  await fs.writeFile(CONFIG_PATH, stringify(merged as any), 'utf-8')
}

/** 递归合并两个对象——对于普通对象递归合并，其他值直接覆盖 */
function deepMerge(target: Record<string, unknown>, source: Record<string, unknown>): Record<string, unknown> {
  const result: Record<string, unknown> = { ...target }
  for (const key of Object.keys(source)) {
    const val = source[key]
    if (
      val !== null &&
      typeof val === 'object' &&
      !Array.isArray(val) &&
      typeof result[key] === 'object' &&
      result[key] !== null &&
      !Array.isArray(result[key])
    ) {
      result[key] = deepMerge(result[key] as Record<string, unknown>, val as Record<string, unknown>)
    } else {
      result[key] = val
    }
  }
  return result
}

export function resolveConfigPaths(config: Record<string, unknown>): ResolvedConfigPaths {
  const paths = (config.paths ?? {}) as Record<string, unknown>
  const runtimeBase = process.cwd()
  const rootValue = typeof paths.root === 'string' && paths.root.trim()
    ? paths.root.trim()
    : '.'
  const libraryValue = typeof paths.library === 'string' && paths.library.trim()
    ? paths.library.trim()
    : 'library'

  const root = path.resolve(runtimeBase, rootValue)
  const library = path.isAbsolute(libraryValue)
    ? path.normalize(libraryValue)
    : path.resolve(root, libraryValue)
  const databaseValue = typeof paths.database === 'string' && paths.database.trim()
    ? paths.database.trim()
    : ''
  const database = databaseValue
    ? path.isAbsolute(databaseValue)
      ? path.normalize(databaseValue)
      : path.resolve(root, databaseValue)
    : undefined

  return {
    root,
    library,
    database,
    runtime_base: runtimeBase,
    config_path: CONFIG_PATH,
  }
}
