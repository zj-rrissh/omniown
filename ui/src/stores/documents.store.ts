import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  fetchDocuments,
  fetchDocument,
  type DocumentSummary,
  type DocumentDetail,
} from '../services/documents.service'

const PAGE_SIZE = 20

export const useDocumentsStore = defineStore('documents', () => {
  const items = ref<DocumentSummary[]>([])      // 已加载的全部文档
  const selected = ref<DocumentDetail | null>(null)
  const totalCount = ref(0)                      // 服务端总文档数
  const folderFilter = ref<'all' | 'public' | 'private'>('all')
  const loading = ref(false)                     // 首次加载
  const loadingMore = ref(false)                 // 加载更多
  const error = ref<string | null>(null)

  /** 已加载且匹配当前筛选的文档 */
  const filteredItems = computed(() => {
    if (folderFilter.value === 'all') return items.value
    return items.value.filter(d => d.folderType === folderFilter.value)
  })

  /** 是否还有更多未加载的数据 */
  const hasMore = computed(() => items.value.length < totalCount.value)

  /** 首次加载前 20 条 */
  async function loadInitial() {
    loading.value = true
    error.value = null
    try {
      const res = await fetchDocuments(PAGE_SIZE, 0)
      items.value = res.documents
      totalCount.value = res.total
    } catch (e: any) {
      error.value = e?.message ?? '加载失败'
    } finally {
      loading.value = false
    }
  }

  /** 加载下一页并追加 */
  async function loadMore() {
    if (loadingMore.value || !hasMore.value) return
    loadingMore.value = true
    try {
      const res = await fetchDocuments(PAGE_SIZE, items.value.length)
      items.value = [...items.value, ...res.documents]
      totalCount.value = res.total
    } catch (e: any) {
      error.value = e?.message ?? '加载更多失败'
    } finally {
      loadingMore.value = false
    }
  }

  /** 重新加载（筛选变更、文件变更时） */
  async function reload() {
    items.value = []
    totalCount.value = 0
    await loadInitial()
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
  }

  return {
    items, selected, totalCount, folderFilter, loading, loadingMore, error,
    filteredItems, hasMore,
    loadInitial, loadMore, reload, selectDocument, setFilter,
  }
})
