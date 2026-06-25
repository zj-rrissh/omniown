import { spawn, ChildProcess } from 'child_process'
import { fileURLToPath } from 'url'
import path from 'path'
import { loadConfig, resolveConfigPaths } from './config/index.js'
import { buildOmniownArgs, resolveDbPath, resolveOmniownBinary } from './utils/omniown-cli.js'
import { emitFileChange } from './services/events.service.js'
import { clearDocStatsCache } from './services/search.service.js'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const projectRoot = path.resolve(__dirname, '..')

let watchProcess: ChildProcess | null = null

function configuredLibraryFrom(config: Record<string, unknown>): string | undefined {
  const paths = (config.paths ?? {}) as Record<string, unknown>
  const hasLibrary = typeof paths.library === 'string' && paths.library.trim()
  const hasRoot = typeof paths.root === 'string' && paths.root.trim()
  return hasLibrary || hasRoot ? resolveConfigPaths(config).library : undefined
}

async function resolveLibraryFromConfig(): Promise<string | undefined> {
  const config = await loadConfig()
  const configuredLibrary = configuredLibraryFrom(config)
  if (configuredLibrary) return configuredLibrary

  const dbPath = resolveDbPath(projectRoot)
  return dbPath ? path.join(path.dirname(dbPath), 'library') : undefined
}

function spawnWatch(library?: string): ChildProcess {
  const bin = resolveOmniownBinary()
  const args = buildOmniownArgs('watch', [], { dbPath: resolveDbPath(projectRoot), library })

  console.log('[watch] 启动:', bin, args.join(' '))

  const child = spawn(bin, args, {
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env },
  })

  let ready = false
  child.stdout?.on('data', (data: Buffer) => {
    const lines = data.toString().split('\n').filter(Boolean)
    for (const line of lines) {
      if (!ready) {
        try {
          const info = JSON.parse(line)
          if (info.status === 'watching') {
            ready = true
            console.log('[watch] 就绪, library:', info.library, ', db:', info.db_path)
            continue
          }
        } catch {
          // Non-JSON stdout is logged below.
        }
      }

      // 检测文件变更事件 → 通知前端 + 清除文档统计缓存
      if (line.includes('已索引') || line.includes('索引完成')) {
        clearDocStatsCache()
        emitFileChange(`file-added: ${line}`)
      } else if (line.includes('已删除记录')) {
        clearDocStatsCache()
        emitFileChange(`file-deleted: ${line}`)
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
    if (watchProcess === child) {
      watchProcess = null
    }
  })

  return child
}

export async function startWatchFromConfig(): Promise<void> {
  stopWatch()
  watchProcess = spawnWatch(await resolveLibraryFromConfig())
}

export async function restartWatchFromConfig(): Promise<void> {
  console.log('[watch] 重新加载配置并重启监听')
  await startWatchFromConfig()
}

export function stopWatch(): void {
  const child = watchProcess
  watchProcess = null

  if (child && !child.killed) {
    console.log('[watch] 停止旧监听进程')
    child.kill()
  }
}
