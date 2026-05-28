// ============================================================
// 配置管理模块
// ============================================================
//
// 提供统一的配置读取/写入函数，供路由层调用。
// 路由层不应该直接操作文件系统，而是通过这里的函数。
//
// ============================================================

import fs from 'fs/promises'
import path from 'path'
import { fileURLToPath } from 'url'
import { parse, stringify } from '@iarna/toml'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// 项目根目录 = server/src/config/ → 向上两级到 server/ ../
// 但实际配置在项目根目录下的 config/ 中
// 所以最终路径是 ../../config/omniown.toml（相对于 server/src/config/）
// 等价于项目根目录 /config/omniown.toml
//
// TODO: 确认你的配置文件实际放在哪
const ROOT = path.resolve(__dirname, '../..')
const CONFIG_PATH = path.join(ROOT, 'omniown.toml')

// --- 读取配置（TOML） ---
//
// 读取并解析 TOML 配置文件。
// 使用 @iarna/toml 提供的 parse/stringify 功能。

export async function loadConfig(): Promise<Record<string, unknown>> {
  try {
    const content = await fs.readFile(CONFIG_PATH, 'utf-8')
    return parse(content) as Record<string, unknown>
  } catch {
    // 文件不存在 → 返回空对象
    return {}
  }
}

// --- 写入配置 ---

export async function saveConfig(config: unknown): Promise<void> {
  // createDirAll: 确保目录存在
  await fs.mkdir(path.dirname(CONFIG_PATH), { recursive: true })
  // 写入 TOML 文件（覆盖）
  await fs.writeFile(CONFIG_PATH, stringify(config as any), 'utf-8')
}
