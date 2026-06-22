import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  fetchDocuments,
  fetchDocument,
  type DocumentSummary,
  type DocumentDetail,
} from '../services/documents.service'

export const useDocumentsStore = defineStore('documents', () => {
  const items = ref<DocumentSummary[]>([])
  const selected = ref<DocumentDetail | null>(null)
  const page = ref(1)
  const perPage = 20
  const folderFilter = ref<'all' | 'public' | 'private'>('all')
  const loading = ref(false)
  const error = ref<string | null>(null)

  const filteredItems = computed(() => {
    if (folderFilter.value === 'all') return items.value
    return items.value.filter(d => d.folderType === folderFilter.value)
  })

  const totalCount = computed(() => filteredItems.value.length)

  const totalPages = computed(() => Math.max(1, Math.ceil(totalCount.value / perPage)))

  const pagedItems = computed(() => {
    const start = (page.value - 1) * perPage
    return filteredItems.value.slice(start, start + perPage)
  })

  async function loadDocuments() {
    loading.value = true
    error.value = null
    try {
      items.value = await fetchDocuments(200)
    } catch (e: any) {
      error.value = e?.message ?? '加载失败'
    } finally {
      loading.value = false
    }
  }

  async function selectDocument(id: number) {
    try {
      selected.value = await fetchDocument(id)
    } catch (e: any) {
      error.value = e?.message ?? '加载详情失败'
    }
  }

  function setFilter(f: 'all' | 'public' | 'private') {
    folderFilter.value = f
    page.value = 1
  }

  function prevPage() {
    if (page.value > 1) page.value--
  }

  function nextPage() {
    if (page.value < totalPages.value) page.value++
  }

  function setPage(p: number) {
    page.value = p
  }

  return {
    items, selected, totalCount, page, perPage, folderFilter, loading, error,
    totalPages, pagedItems,
    loadDocuments, selectDocument, setFilter, prevPage, nextPage, setPage,
  }
})
