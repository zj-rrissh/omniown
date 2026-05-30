export interface ApiError {
  error?: {
    message?: string
  }
}

export async function getJson<T>(url: string): Promise<T> {
  let response: Response
  try {
    response = await fetch(url)
  } catch {
    throw new Error(`Cannot reach ${url}. Is \`cargo run -- serve\` running?`)
  }

  const text = await response.text()

  if (!text.trim()) {
    throw new Error(`Empty response from ${url}`)
  }

  let data: T & ApiError
  try {
    data = JSON.parse(text) as T & ApiError
  } catch {
    throw new Error(`Invalid JSON from ${url}`)
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