import { spawn } from 'child_process'
import { existsSync, readdirSync, readFileSync } from 'fs'
import { fileURLToPath } from 'url'
import path from 'path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const serverRoot = path.resolve(__dirname, '..', '..')
const repoOrResourceRoot = path.resolve(serverRoot, '..')

function executableNames(base: string): string[] {
  return process.platform === 'win32' ? [`${base}.exe`, base] : [base, `${base}.exe`]
}

function firstExisting(paths: string[]): string | undefined {
  return paths.find((candidate) => existsSync(candidate))
}

function findBundledSidecar(root: string): string | undefined {
  const dirs = [path.join(root, 'binaries'), root]
  for (const dir of dirs) {
    try {
      if (!existsSync(dir)) continue
      const match = readdirSync(dir).find((file) => {
        if (!file.startsWith('omniown-')) return false
        if (process.platform === 'win32') return file.endsWith('.exe')
        return !file.endsWith('.exe')
      })
      if (match) return path.join(dir, match)
    } catch {
      // Ignore unreadable candidate directories.
    }
  }
  return undefined
}

export function resolveOmniownBinary(): string {
  const envBin = process.env.OMNIOWN_BIN?.trim().replace(/^"|"$/g, '')
  if (envBin && existsSync(envBin)) return envBin

  const devCandidates = [
    ...executableNames('omniown').map((name) => path.join(repoOrResourceRoot, 'target', 'debug', name)),
    ...executableNames('omniown').map((name) => path.join(repoOrResourceRoot, 'target', 'release', name)),
  ]
  const installedCandidates = executableNames('omniown').flatMap((name) => [
    path.join(repoOrResourceRoot, name),
    path.join(serverRoot, name),
  ])

  return (
    firstExisting([...devCandidates, ...installedCandidates]) ??
    findBundledSidecar(repoOrResourceRoot) ??
    'omniown'
  )
}

export function resolveDbPath(projectRoot = serverRoot): string {
  let url = process.env.DATABASE_URL || ''
  if (!url) {
    try {
      const envFile = path.join(projectRoot, '.env')
      if (existsSync(envFile)) {
        const content = readFileSync(envFile, 'utf-8')
        const match = content.match(/^DATABASE_URL\s*=\s*(.+)$/m)
        if (match) url = match[1].trim().replace(/^"|"$/g, '')
      }
    } catch {
      // Ignore missing or unreadable dotenv files.
    }
  }
  if (!url.startsWith('file:')) return ''

  let dbPath = url.slice(5)
  if (!path.isAbsolute(dbPath)) {
    dbPath = path.resolve(projectRoot, 'prisma', dbPath)
  }
  return dbPath
}

export function buildOmniownArgs(
  command: string,
  args: string[] = [],
  options: { dbPath?: string; library?: string } = {},
): string[] {
  const resolved = [command, ...args]
  const dbPath = options.dbPath ?? resolveDbPath()
  if (dbPath) resolved.push('--db-path', dbPath)
  if (options.library) resolved.push('--library', path.resolve(options.library))
  return resolved
}

export function runOmniown(
  args: string[],
): Promise<{ stdout: string; stderr: string }> {
  const bin = resolveOmniownBinary()
  return new Promise((resolve, reject) => {
    const child = spawn(bin, args, {
      stdio: ['ignore', 'pipe', 'pipe'],
      env: { ...process.env },
    })

    let stdout = ''
    let stderr = ''
    child.stdout?.on('data', (data: Buffer) => {
      stdout += data.toString()
    })
    child.stderr?.on('data', (data: Buffer) => {
      stderr += data.toString()
    })
    child.on('error', reject)
    child.on('close', (code) => {
      if (code === 0) {
        resolve({ stdout, stderr })
      } else {
        const err = new Error(stderr || `omniown exited with code ${code}`)
        ;(err as Error & { stdout?: string; stderr?: string; code?: number }).stdout = stdout
        ;(err as Error & { stdout?: string; stderr?: string; code?: number }).stderr = stderr
        ;(err as Error & { stdout?: string; stderr?: string; code?: number }).code = code ?? undefined
        reject(err)
      }
    })
  })
}
