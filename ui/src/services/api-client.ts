export interface ApiError {
  error?: {
    message?: string
  }
}

const API_BASE_URL =
  import.meta.env.VITE_API_BASE_URL ?? (import.meta.env.DEV ? '' : 'http://127.0.0.1:3001')

export function apiUrl(url: string): string {
  if (/^https?:\/\//i.test(url)) {
    return url
  }
  const path = url.startsWith('/') ? url : `/${url}`
  return `${API_BASE_URL}${path}`
}

export async function getJson<T>(url: string): Promise<T> {
  const requestUrl = apiUrl(url)
  let response: Response
  try {
    response = await fetch(requestUrl)
  } catch {
    throw new Error(`Cannot reach ${requestUrl}. Is the OmniOwn API running?`)
  }

  const text = await response.text()

  if (!text.trim()) {
    throw new Error(`Empty response from ${requestUrl}`)
  }

  let data: T & ApiError
  try {
    data = JSON.parse(text) as T & ApiError
  } catch {
    throw new Error(`Invalid JSON from ${requestUrl}`)
  }

  if (!response.ok) {
    throw new Error(data.error?.message ?? 'Request failed')
  }

  return data
}

export function withContext(error: unknown, context: string): Error {
  const message = error instanceof Error ? error.message : 'Request failed'
  return new Error(`${context}: ${message}`)
}
