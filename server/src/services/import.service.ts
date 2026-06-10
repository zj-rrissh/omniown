// 文件导入服务 — 通过 child_process 调用 Rust CLI

import path from 'path'
import { buildOmniownArgs, runOmniown } from '../utils/omniown-cli.js'
import { loadConfig, resolveConfigPaths } from '../config/index.js'

export interface ImportResult {
  success: boolean
  filename: string
  message: string
  storedPath?: string
  error?: string
}

export async function importFile(filePath: string): Promise<ImportResult> {
  try {
    const appConfig = await loadConfig()
    const configPaths = resolveConfigPaths(appConfig)
    const { stdout, stderr } = await runOmniown(
      buildOmniownArgs('process', [filePath], { library: configPaths.library })
    )

    if (stderr) {
      console.warn('[import] stderr:', stderr)
    }

    return {
      success: true,
      filename: extractFilename(filePath),
      message: '导入成功',
      storedPath: parseStoredPath(stdout),
    }
  } catch (err) {
    return {
      success: false,
      filename: extractFilename(filePath),
      message: '导入失败',
      error: err instanceof Error ? err.message : String(err),
    }
  }
}

export async function getCliVersion(): Promise<string> {
  try {
    const { stdout, stderr } = await runOmniown(['--version'])
    return stdout.trim() || stderr.trim() || 'unknown'
  } catch {
    return 'unknown (omniown CLI not found)'
  }
}

function extractFilename(filePath: string): string {
  return path.basename(filePath)
}

function parseStoredPath(output: string): string | undefined {
  const lines = output.split('\n')
  for (const line of lines) {
    const trimmed = line.trim()
    if (trimmed.startsWith('library/') || trimmed.startsWith('library\\')) {
      return trimmed
    }
  }
  return undefined
}
