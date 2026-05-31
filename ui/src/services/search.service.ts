export interface SearchResult {
  id: number
  filename: string
  storedPath: string
  folderType: string
  category: string
  snippet: string | null
  rank: number
  updatedAt: string
}

import { getJson, withContext } from './api-client'

export async function searchDocuments(query: string, limit = 50): Promise<SearchResult[]> {
  const params = new URLSearchParams({ q: query, limit: String(limit) })
  try {
    const data = await getJson<{ results: SearchResult[] }>(`/api/search?${params}`)
    return data.results
  } catch (error) {
    throw withContext(error, 'Search request failed')
  }
}