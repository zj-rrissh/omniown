import { defineStore } from 'pinia'
import { ref } from 'vue'
import { searchDocuments, type SearchResult } from '../services/search.service'

export const useSearchStore = defineStore('search', () => {
  const query = ref('')
  const results = ref<SearchResult[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function search(term: string) {
    query.value = term
    loading.value = true
    error.value = null
    try {
      results.value = await searchDocuments(term)
    } catch (e: any) {
      error.value = e?.message ?? '搜索失败'
    } finally {
      loading.value = false
    }
  }

  function clear() {
    query.value = ''
    results.value = []
    error.value = null
  }

  return { query, results, loading, error, search, clear }
})
