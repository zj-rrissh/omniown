export interface StatusResponse {
  database: string
  root: string
  schema: {
    current_version: number
    pending_migrations: number
  }
  documents: {
    total: number
    public: number
    private: number
    indexed: number
    failed: number
  }
}

export interface DocumentSummary {
  id: number
  filename: string
  stored_path: string
  folder_type: string
  category: string
  risk_level: string
  processing_status: string
  updated_at: string
  file_ext: string | null
  file_size: number | null
}

export interface DocumentDetail extends DocumentSummary {
  original_path: string | null
  domain: string
  doc_type: string
  content: string | null
  summary: string | null
  tags: string | null
  privacy_score: number
  summary_status: string
  created_at: string
  imported_at: string
}

export interface SearchResult {
  id: number
  filename: string
  stored_path: string
  folder_type: string
  category: string
  snippet: string | null
  rank: number
  updated_at: string
}

interface ApiError {
  error?: {
    message?: string
  }
}

/** API 基础 URL — Tauri 模式下与 omniown 后端同源，用相对路径即可 */
async function getJson<T>(url: string): Promise<T> {
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

export function fetchStatus(): Promise<StatusResponse> {
  return getJson<StatusResponse>('/api/status').catch((error: unknown) => {
    throw withContext(error, 'Failed to load status')
  })
}

export async function fetchDocuments(limit = 50): Promise<DocumentSummary[]> {
  try {
    const data = await getJson<{ documents: DocumentSummary[] }>(`/api/documents?limit=${limit}`)
    return data.documents
  } catch (error) {
    throw withContext(error, 'Failed to load document list')
  }
}

export async function searchDocuments(query: string, limit = 50): Promise<SearchResult[]> {
  const params = new URLSearchParams({ q: query, limit: String(limit) })
  try {
    const data = await getJson<{ results: SearchResult[] }>(`/api/search?${params}`)
    return data.results
  } catch (error) {
    throw withContext(error, 'Search request failed')
  }
}

export async function fetchDocument(id: number): Promise<DocumentDetail> {
  try {
    const data = await getJson<{ document: DocumentDetail }>(`/api/documents/${id}`)
    return data.document
  } catch (error) {
    throw withContext(error, `Failed to load document #${id}`)
  }
}

function withContext(error: unknown, context: string): Error {
  const message = error instanceof Error ? error.message : 'Request failed'
  return new Error(`${context}: ${message}`)
}
