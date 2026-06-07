// OmniOwn API 入口 — 路由挂载与服务器启动

import express from 'express'
import cors from 'cors'
import { execSync, spawn, ChildProcess } from 'child_process'
import { existsSync } from 'fs'
import { fileURLToPath } from 'url'
import path from 'path'
import { buildOmniownArgs, resolveDbPath, resolveOmniownBinary } from './utils/omniown-cli.js'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const app = express()

// --- 全局中间件 ---
app.use(cors())
app.use(express.json())

// --- 数据库初始化 ---
// 首次启动 / 数据库不存在时自动创建表
const projectRoot = path.resolve(__dirname, '..')
// 打包后 schema 在 server/dist/prisma/，开发时在 server/prisma/
const schema = existsSync(path.resolve(__dirname, 'prisma', 'schema.prisma'))
  ? path.resolve(__dirname, 'prisma', 'schema.prisma')       // 打包模式: server/dist/prisma/
  : path.resolve(projectRoot, 'prisma', 'schema.prisma')     // 开发模式: server/prisma/
process.env.PRISMA_SCHEMA_PATH = schema
try {
  execSync(`npx prisma db push --skip-generate --schema="${schema}"`, {
    cwd: projectRoot,
    stdio: 'pipe',
    env: { ...process.env, PRISMA_SCHEMA_PATH: schema }
  })
  console.log('[db] Schema 已同步')
} catch (err) {
  const msg = err instanceof Error ? (err as any).stderr?.toString() ?? err.message : String(err)
  console.warn('[db] Schema 同步警告:', msg.slice(0, 200))
}

// 设置 WAL 模式 — 与 Rust rusqlite 端统一 journal 模式，避免并发访问冲突
try {
  const dbUrl = process.env.DATABASE_URL || ''
  if (dbUrl.startsWith('file:')) {
    execSync(`sqlite3 "${path.resolve(projectRoot, 'prisma', dbUrl.slice(5))}" "PRAGMA journal_mode=WAL"`, {
      stdio: 'pipe'
    })
  }
} catch { /* sqlite3 不可用时忽略 */ }

// FTS5 虚拟表需要手动创建（Prisma 不支持）
import { initFts5 } from './db/setup-fts.js'
await initFts5()

// --- 启动文件夹监听 (omniown watch) ---
let watchProcess: ChildProcess | null = null

function spawnWatch(library?: string) {
  const bin = resolveOmniownBinary()
  const args = buildOmniownArgs('watch', [], { dbPath: resolveDbPath(projectRoot), library })

  console.log('[watch] 启动:', bin, args.join(' '))

  const child = spawn(bin, args, {
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env },
  })

  // 监听就绪信号（stdout 第一行 JSON）
  let ready = false
  child.stdout?.on('data', (data: Buffer) => {
    const lines = data.toString().split('\n').filter(Boolean)
    for (const line of lines) {
      if (!ready) {
        try {
          const info = JSON.parse(line)
          if (info.status === 'watching') {
            ready = true
            console.log('[watch] 就绪, inbox:', info.inbox, ', db:', info.db_path)
            continue
          }
        } catch { /* 非 JSON 行，跳过 */ }
      }
      console.log('[watch]', line)
    }
  })

  child.stderr?.on('data', (data: Buffer) => {
    console.error('[watch]', data.toString().trimEnd())
  })

  child.on('error', (err) => {
    console.warn('[watch] 启动失败:', err.message)
  })

  child.on('exit', (code, signal) => {
    console.log('[watch] 进程退出, 退出码:', code, ', 信号:', signal)
    watchProcess = null
  })

  return child
}

const { loadConfig } = await import('./config/index.js')
const appConfig = await loadConfig()
const configPaths = (appConfig.paths ?? {}) as Record<string, string>
watchProcess = spawnWatch(configPaths.library)

// 进程退出时清理
process.on('exit', () => {
  if (watchProcess && !watchProcess.killed) {
    watchProcess.kill()
  }
})
process.on('SIGTERM', () => {
  if (watchProcess && !watchProcess.killed) {
    watchProcess.kill()
  }
  process.exit(0)
})

// --- 路由挂载 ---

import { router as statusRouter } from './api/status.js'
import { router as documentsRouter } from './api/documents.js'
import { router as configRouter } from './api/config.js'
import { router as searchRouter } from './api/search.js'

app.use('/api', statusRouter)          // GET  /api/status
app.use('/api/documents', documentsRouter)  // GET  /api/documents
                                       // GET  /api/documents/:id
app.use('/api/config', configRouter)   // GET  /api/config
                                       // PUT  /api/config
app.use('/api/search', searchRouter)   // GET  /api/search?q=

// --- 启动服务器 ---

app.listen(3001, () => {
  console.log('OmniOwn API: http://127.0.0.1:3001')
})
