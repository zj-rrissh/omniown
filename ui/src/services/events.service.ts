/**
 * SSE 事件客户端 — 监听 library 文件变更，通知组件重新加载
 */

export interface WatchEvent {
  type: 'file-change'
  message: string
  timestamp: number
}

type Listener = (event: WatchEvent) => void

const API_BASE_URL =
  import.meta.env.VITE_API_BASE_URL ?? (import.meta.env.DEV ? '' : 'http://127.0.0.1:3001')

const listeners = new Set<Listener>()
let eventSource: EventSource | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null

function connect(): void {
  if (eventSource) return

  const url = `${API_BASE_URL}/api/events`
  eventSource = new EventSource(url)

  eventSource.onopen = () => {
    console.log('[events] SSE 已连接')
  }

  eventSource.onmessage = (e) => {
    try {
      const data = JSON.parse(e.data) as WatchEvent | { type: 'connected' }
      if (data.type === 'file-change') {
        const event = data as WatchEvent
        for (const listener of listeners) {
          listener(event)
        }
      }
    } catch {
      // 忽略非 JSON 消息（如心跳）
    }
  }

  eventSource.onerror = () => {
    console.warn('[events] SSE 连接断开，3s 后重试')
    disconnect()
    reconnectTimer = setTimeout(connect, 3000)
  }
}

function disconnect(): void {
  if (eventSource) {
    eventSource.close()
    eventSource = null
  }
}

/**
 * 订阅文件变更事件。
 * 首次订阅时自动建立 SSE 连接。
 * 返回取消订阅函数。
 */
export function onFileChange(listener: Listener): () => void {
  listeners.add(listener)
  connect()

  return () => {
    listeners.delete(listener)
    if (listeners.size === 0) {
      disconnect()
      if (reconnectTimer) {
        clearTimeout(reconnectTimer)
        reconnectTimer = null
      }
    }
  }
}
