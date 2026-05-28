// ============================================================
// /api/config — 配置读取/写入
// ============================================================
//
// 这个模块管理 OmniOwn 的配置。
// 配置文件是 TOML 格式（和 Rust 版本保持一致），路径为 config/omniown.toml。
//
// TOML vs JSON：
// TOML 是人类可读性更好的配置文件格式。
// Node.js 没有内置 TOML 解析器，需要安装 toml 包：
//   npm install toml
//
// ============================================================

import { Router } from 'express'
import fs from 'fs/promises'  // fs/promises 是 Promise 版本的 fs 模块
import path from 'path'        // path 用于处理文件路径
import { fileURLToPath } from 'url'  // 用于 ES Module 中获取当前文件路径

export const router = Router()

// --- 获取配置文件路径 ---
//
// 在 CommonJS 中，可以用 __dirname 获取当前文件所在目录。
// 在 ES Module 中，需要用 import.meta.url + fileURLToPath 手动计算。
//
// 配置路径策略（优先级从高到低）：
// 1. 环境变量 OMNIOWN_CONFIG_PATH
// 2. 默认路径: 项目根目录下的 config/omniown.toml

// 获取当前文件的目录路径（ES Module 写法）
// import.meta.url 是当前文件的 file:// URL
// fileURLToPath() 把它转成普通路径
// path.dirname() 取目录名
const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// 项目根目录 = server/src/config/index.ts → 往上两级到 server/
// 实际项目中应该用环境变量或固定约定
// TODO: 根据你的目录结构调整这个路径
const configDir = process.env.OMNIOWN_CONFIG_DIR
  || path.resolve(__dirname, '../../config')
const configPath = path.join(configDir, 'omniown.toml')

// --- TOML 解析器准备 ---
//
// 由于 TOML 不是 Node.js 内置格式，有两种方案：
//
// 方案 A：安装 toml 包（推荐）
//   npm install toml
//   import toml from 'toml'
//
// 方案 B：先用 JSON 格式写配置（简单，但和 Rust 不兼容）
//   配置文件改为 config/omniown.json
//   直接用 fs.readFile + JSON.parse
//
// 下面的代码使用方案 B 的 JSON 格式作为演示，
// 你可以改成 TOML 版本。

// ------- GET /api/config -------

router.get('/', async (_req, res) => {
  try {
    // 尝试读取配置文件
    // await fs.readFile() 返回 Buffer，用 .toString() 或 utf-8 参数转字符串
    const content = await fs.readFile(configPath, 'utf-8')

    // JSON.parse() 把 JSON 字符串转成 JavaScript 对象
    const config = JSON.parse(content)

    // 返回配置对象
    // 路线：不要返回 api_key 等敏感字段
    // 如果 config.ai 和 config.ai.api_key 存在，删除 api_key
    if (config.ai?.api_key) {
      config.ai.api_key = '***'
    }

    res.json(config)
  } catch (err) {
    // 文件不存在的错误码：ENOENT (Error NO ENTity)
    if (err instanceof Error && (err as NodeJS.ErrnoException).code === 'ENOENT') {
      // 配置文件不存在不是错误，返回空配置
      res.json({})
      return
    }

    // 其他错误（如 JSON 格式错误）返回 500
    const msg = err instanceof Error ? err.message : '读取配置失败'
    res.status(500).json({ error: msg })
  }
})

// ------- PUT /api/config — 更新配置 -------

// PUT 是幂等的：多次调用结果相同
// POST 是非幂等的：多次调用可能产生不同结果
// 这里用 PUT 表示"替换整个配置"

router.put('/', async (req, res) => {
  try {
    // req.body 是客户端传来的新配置（JSON 对象）
    // 需要 express.json() 中间件才能解析

    const newConfig = req.body

    // 字段校验：检查必要字段是否存在
    // 这里可以加自己的校验逻辑
    // if (!newConfig.ai || !newConfig.ai.base_url) {
    //   res.status(400).json({ error: '缺少 ai.base_url 字段' })
    //   return
    // }

    // 把配置对象写回文件
    // JSON.stringify(obj, null, 2) 格式化输出（缩进 2 空格）
    await fs.writeFile(configPath, JSON.stringify(newConfig, null, 2), 'utf-8')

    // 204 No Content — 成功但无返回体
    // 也可以返回 200 + 更新后的配置
    res.status(204).send()
  } catch (err) {
    const msg = err instanceof Error ? err.message : '写入配置失败'
    res.status(500).json({ error: msg })
  }
})

// ============================================================
// 学习笔记
// ============================================================
//
// fs/promises vs fs:
// fs.readFile(path, callback)    — 回调风格（旧）
// fs.promises.readFile(path)     — Promise 风格（推荐）
// fs.readFileSync(path)          — 同步阻塞（不推荐在服务器用）
//
// JSON.stringify(obj, null, 2) 的第二个参数是 replacer 函数，
// 可以用来过滤敏感字段：
//   JSON.stringify(obj, (key, val) => {
//     if (key === 'api_key') return undefined  // 跳过 api_key
//     return val
//   }, 2)
//
// NodeJS.ErrnoException 是 Node.js 的错误类型，
// 包含 code（错误码如 'ENOENT'）、errno、syscall 等字段。
// TypeScript 中需要 (err as NodeJS.ErrnoException).code 来访问。
//
// 配置分层：
// 1. 硬编码默认值（最低优先级）
// 2. JSON/TOML 文件（中等优先级）
// 3. 环境变量（最高优先级）
// 这叫 "12-Factor App" 配置管理原则
