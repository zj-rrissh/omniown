import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  aiSearchDocuments,
  getAiSearchUrl,
  getSearchUrl,
  searchDocuments,
  type SearchResult,
  type SearchTrace,
} from '../services/search.service'

export type SearchMode = 'normal' | 'ai'

export const useSearchStore = defineStore('search', () => {
  const query = ref('')
  const results = ref<SearchResult[]>([])
  const trace = ref<SearchTrace | null>(null)
  const mode = ref<SearchMode>('normal')
  const requestUrl = ref('')
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function search(term: string, searchMode: SearchMode = 'normal') {
    query.value = term
    mode.value = searchMode
    requestUrl.value = searchMode === 'ai'
      ? getAiSearchUrl(term)
      : getSearchUrl(term)
    loading.value = true
    error.value = null
    trace.value = null
    try {
      const response = searchMode === 'ai'
        ? await aiSearchDocuments(term)
        : await searchDocuments(term)
      results.value = response.results
      requestUrl.value = response.requestUrl
      trace.value = {
        ...(response.trace ?? {}),
        requestUrl: response.requestUrl,
      }
    } catch (e: any) {
      error.value = e?.message ?? '搜索失败'
      trace.value = {
        error: error.value ?? undefined,
      }
    } finally {
      loading.value = false
    }
  }

  function clear() {
    query.value = ''
    results.value = []
    trace.value = null
    mode.value = 'normal'
    requestUrl.value = ''
    error.value = null
  }

  function clearOutput() {
    results.value = []
    trace.value = null
    requestUrl.value = ''
    error.value = null
  }

  return { query, results, trace, mode, requestUrl, loading, error, search, clear, clearOutput }
})
