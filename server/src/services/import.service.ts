// ============================================================
// 文件导入服务
// ============================================================
//
// 这个服务把文件导入流程交给 Rust CLI 执行。
// Rust 擅长二进制文件解析（PDF/DOCX/XLSX），Node.js 不适合做。
//
// 流程：
//   importFile("/home/user/inbox/note.md")
//      ↓
//   child_process.exec("omniown process /home/user/inbox/note.md")
//      ↓
//   Rust CLI → extractor → classifier → storage → db insert
//      ↓
//   返回处理结果
//
// 为什么用 child_process 而不是 HTTP 调用？
//   1. 文件已存在磁盘上，不需要网络传输
//   2. CLI 跨设备兼容（Windows/macOS/Linux）
//   3. 路径问题：CLI 直接操作本地文件系统
//
// ============================================================

import { exec } from 'child_process'
import { promisify } from 'util'

// promisify 把回调风格的 exec 转成 Promise 版本
const execPromise = promisify(exec)

/**
 * 单个节点处理多个文件
 */
export interface ImportResult {
  success: boolean
  filename: string
  message: string
  storedPath?: string
  error?: string
}

/**
 * 导出一个单文件。
 *
 * @param filePath 要导入的文件绝对路径（必须在 inbox 目录下）
 * @returns 处理结果
 *
 * 示例：
 *   importFile("/home/user/inbox/rust-tutorial.md")
 *   → { success: true, filename: "rust-tutorial.md", storedPath: "library/public/rust-tutorial.md" }
 */
export async function importFile(filePath: string): Promise<ImportResult> {
  try {
    // 调用 Rust CLI
    // "omniown" 是 rust 二进制，需在 PATH 中（或指明路径）
    // "process" 是子命令（src/main.rs 中 "process" 分支）
    //
    // exec 的 stdout 返回 stdout 字符串，stderr 返回 stderr 字符串
    const { stdout, stderr } = await execPromise(
      `omniown process "${filePath}"`
    )

    // stderr 可能有警告信息但不一定是错误
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
    // exec 抛出异常表示进程退出码非 0
    return {
      success: false,
      filename: extractFilename(filePath),
      message: '导入失败',
      error: err instanceof Error ? err.message : String(err),
    }
  }
}

/**
 * 获取 Rust CLI 的版本信息（用于健康检查）
 */
export async function getCliVersion(): Promise<string> {
  try {
    const { stdout } = await execPromise('omniown --version')
    return stdout.trim()
  } catch {
    return 'unknown (omniown CLI not found)'
  }
}

// ============================================================
// 辅助函数
// ============================================================

/**
 * 从文件路径中提取文件名。
 *
 *   /home/user/inbox/note.md → "note.md"
 *   C:\Users\test\inbox\note.md → "note.md"
 *
 * path.basename() 在不同操作系统下表现一致。
 */
function extractFilename(filePath: string): string {
  // 动态 import 避免 Node.js 内置模块在 bundle 时出现问题
  // path 是 Node.js 内置模块，不需要安装
  const path = require('path') as typeof import('path')
  return path.basename(filePath)
}

/**
 * 从 CLI 输出中解析 stored_path。
 *
 * Rust CLI 的 stdout 可能包含多行输出，如：
 *   ✅ 处理完成: [note.md] folder=public category=notes id=5
 *   路径：library/public/note.md
 *
 * 这里简单处理：找 "library/" 开头的行
 */
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
