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

export interface StrategyCall {
  strategy: string
  params: Record<string, string>
}

export interface StrategyExecutionTrace {
  strategy: string
  params: Record<string, string>
  status: 'fulfilled' | 'rejected'
  resultCount: number
  results: SearchResult[]
  error?: string
}

export interface SearchTrace {
  model?: string
  baseUrl?: string
  prompt?: string
  requestUrl?: string
  rawResponse?: string
  selectedStrategies?: StrategyCall[]
  strategyResults?: StrategyExecutionTrace[]
  mergedResultCount?: number
  error?: string
}

export interface SearchOptions {
  limit?: number
}

export interface SearchResponse {
  results: SearchResult[]
  trace?: SearchTrace
  requestUrl: string
}

export function getSearchUrl(query: string, options: SearchOptions = {}): string {
  const params = new URLSearchParams({
    q: query,
    limit: String(options.limit ?? 50),
  })

  return `/api/search?${params}`
}

export function getAiSearchUrl(query: string, options: SearchOptions = {}): string {
  const params = new URLSearchParams({
    q: query,
    limit: String(options.limit ?? 50),
    ai: 'true',
  })

  return `/api/search?${params}`
}

export async function searchDocuments(
  query: string,
  options: SearchOptions = {}
): Promise<SearchResponse> {
  const requestUrl = getSearchUrl(query, options)

  try {
    const data = await getJson<Omit<SearchResponse, 'requestUrl'>>(requestUrl)
    return { ...data, requestUrl }
  } catch (error) {
    throw withContext(error, 'Search request failed')
  }
}

export async function aiSearchDocuments(
  query: string,
  options: SearchOptions = {}
): Promise<SearchResponse> {
  const requestUrl = getAiSearchUrl(query, options)

  try {
    const data = await getJson<Omit<SearchResponse, 'requestUrl'>>(requestUrl)
    return { ...data, requestUrl }
  } catch (error) {
    throw withContext(error, 'AI search request failed')
  }
}
