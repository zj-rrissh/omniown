/**
 * SSE 事件服务 — 通知前端 library 文件变更
 *
 * 当 watch 进程检测到文件新增/删除时，通过此模块向所有 SSE 客户端广播。
 */

import { EventEmitter } from 'events'
import type { Response } from 'express'

export type WatchEventType = 'file-change'

export interface WatchEvent {
  type: WatchEventType
  /** 变更描述 */
  message: string
  /** 时间戳 */
  timestamp: number
}

const emitter = new EventEmitter()
const CLIENTS = new Set<Response>()

// 事件名常量
export const EVENT_FILE_CHANGE = 'file-change'

/**
 * 发射文件变更事件，通知所有 SSE 客户端。
 */
export function emitFileChange(message: string): void {
  const event: WatchEvent = {
    type: 'file-change',
    message,
    timestamp: Date.now(),
  }
  const payload = `data: ${JSON.stringify(event)}\n\n`

  for (const res of CLIENTS) {
    try {
      res.write(payload)
    } catch {
      CLIENTS.delete(res)
    }
  }

  // 同时也保留 EventEmitter 方式，供内部监听用
  emitter.emit(EVENT_FILE_CHANGE, event)
}

/**
 * 添加 SSE 客户端连接。
 * 返回一个 cleanup 函数，客户端断开时调用。
 */
export function addSseClient(res: Response): () => void {
  CLIENTS.add(res)

  // 发送初始心跳
  res.write(`data: ${JSON.stringify({ type: 'connected', timestamp: Date.now() })}\n\n`)

  const heartbeat = setInterval(() => {
    try {
      res.write(`:heartbeat\n\n`)
    } catch {
      clearInterval(heartbeat)
      CLIENTS.delete(res)
    }
  }, 15_000)

  return () => {
    clearInterval(heartbeat)
    CLIENTS.delete(res)
  }
}
