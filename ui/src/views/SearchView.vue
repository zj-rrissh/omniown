<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useSearchStore } from '../stores/search.store'
import { useDocumentsStore } from '../stores/documents.store'
import { fetchStatus, type StatusResponse } from '../services/status.service'
import type { DocumentSummary } from '../services/documents.service'
import type { SearchResult } from '../services/search.service'

type ListItem = DocumentSummary | SearchResult

const searchStore = useSearchStore()
const docStore = useDocumentsStore()
const { query } = storeToRefs(searchStore)
const { selected } = storeToRefs(docStore)

const status = ref<StatusResponse | null>(null)
const mode = ref<'documents' | 'search'>('documents')

const items = computed<ListItem[]>(() => {
  if (mode.value === 'search') return searchStore.results
  return docStore.pagedItems
})

const loading = computed(() =>
  mode.value === 'search' ? searchStore.loading : docStore.loading
)

const error = computed(() =>
  mode.value === 'search' ? searchStore.error : docStore.error
)

const isDatabaseEmpty = computed(() => (status.value?.documents.total ?? 0) === 0)

const MAX_CONTENT_LEN = 100_000
const contentDisplay = computed(() => {
  const c = selected.value?.content
  if (!c) return '(无内容)'
  if (c.length > MAX_CONTENT_LEN) return c.slice(0, MAX_CONTENT_LEN) + '\n\n… (截断，超出部分未显示)'
  return c
})

function isSearchItem(item: ListItem): item is SearchResult {
  return 'rank' in item
}

onMounted(async () => {
  try { status.value = await fetchStatus() } catch {}
  await docStore.loadDocuments()
})

async function runSearch() {
  const term = query.value.trim()
  if (!term) {
    mode.value = 'documents'
    await docStore.loadDocuments()
    return
  }
  mode.value = 'search'
  await searchStore.search(term)
}

async function selectDocument(id: number) {
  await docStore.selectDocument(id)
}
</script>

<template>
  <div class="search-view">
    <header class="view-header" data-tauri-drag-region>
      <h1>搜索</h1>
      <span v-if="status" class="count">{{ status.documents.total }} 篇</span>
    </header>

    <form class="search-box" @submit.prevent="runSearch">
      <input
        v-model="query"
        type="search"
        placeholder="输入关键词搜索…"
        autocomplete="off"
      />
      <button type="submit" class="primary" :disabled="loading">搜索</button>
    </form>

    <div v-if="error" class="notice">{{ error }}</div>
    <div v-else-if="isDatabaseEmpty" class="notice info">
      知识库为空。将文件放入 <code>inbox/</code> 后刷新。
    </div>

    <div class="result-list">
      <button
        v-for="item in items" :key="item.id"
        type="button"
        class="result-row"
        :class="{ active: selected?.id === item.id }"
        @click="selectDocument(item.id)"
      >
        <span class="result-name">{{ item.filename }}</span>
        <span v-if="isSearchItem(item)" class="score">Score: {{ item.rank.toFixed(2) }}</span>
        <span class="result-meta">
          {{ item.category }} · {{ item.folderType }}
        </span>
      </button>
      <div v-if="!items.length && !loading" class="empty">无结果</div>
    </div>

    <!-- 详情面板 -->
    <section v-if="selected" class="detail-panel">
      <div class="detail-head">
        <h2>{{ selected.filename }}</h2>
        <button class="close-btn" @click="selected = null">✕</button>
      </div>
      <dl>
        <div><dt>路径</dt><dd>{{ selected.storedPath }}</dd></div>
        <div><dt>类型</dt><dd>{{ selected.folderType }} / {{ selected.category }}</dd></div>
        <div><dt>风险</dt><dd>{{ selected.riskLevel }}</dd></div>
        <div><dt>更新</dt><dd>{{ selected.updatedAt }}</dd></div>
      </dl>
      <pre>{{ contentDisplay }}</pre>
    </section>
  </div>
</template>

<style scoped>
.search-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  color: #e0e0e0;
}

.view-header {
  display: flex;
  align-items: baseline;
  gap: 10px;
  padding: 14px 16px 0;
}
.view-header h1 { font-size: 16px; margin: 0; }
.view-header .count { font-size: 12px; color: #888; }

.search-box {
  display: flex;
  gap: 8px;
  padding: 10px 16px;
}
.search-box input {
  flex: 1; height: 34px;
  padding: 0 10px;
  border: 1px solid rgba(255,255,255,0.1);
  border-radius: 6px;
  background: rgba(255,255,255,0.06);
  color: #e0e0e0; font-size: 13px;
}
.search-box input:focus { outline: none; border-color: #4455cc; }
.search-box button.primary {
  height: 34px; padding: 0 14px;
  border: none; border-radius: 6px;
  background: #4455cc; color: white; font-size: 13px; cursor: pointer;
}
.search-box button.primary:disabled { opacity: 0.6; }

.notice { padding: 8px 16px; font-size: 12px; }
.notice:not(.info) { color: #e05555; }
.notice.info { color: #888; }

.result-list {
  flex: 1; overflow: auto; padding: 0 8px;
}

.result-row {
  display: flex; flex-direction: column; width: 100%;
  padding: 10px 12px; border: 0;
  border-bottom: 1px solid rgba(255,255,255,0.04);
  background: none; color: inherit; text-align: left; cursor: pointer;
}
.result-row:hover, .result-row.active { background: rgba(255,255,255,0.04); }
.result-name { font-size: 13px; }
.result-meta { font-size: 11px; color: #888; margin-top: 2px; }
.score { font-size: 10px; color: #4455cc; }

.empty { padding: 20px; text-align: center; color: #666; font-size: 13px; }

.detail-panel {
  position: absolute; inset: 0;
  background: rgba(20,20,30,0.98);
  display: flex; flex-direction: column; overflow: auto; padding: 16px;
}
.detail-head { display: flex; justify-content: space-between; align-items: start; margin-bottom: 12px; }
.detail-head h2 { font-size: 16px; margin: 0; }
.close-btn { background: none; border: none; color: #888; font-size: 16px; cursor: pointer; }
dl { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin: 0 0 12px; }
dt { font-size: 11px; color: #888; }
dd { font-size: 12px; margin: 0; }
pre {
  flex: 1; overflow: auto; padding: 12px;
  border: 1px solid rgba(255,255,255,0.06); border-radius: 6px;
  background: rgba(0,0,0,0.2);
  font-size: 12px; line-height: 1.5; white-space: pre-wrap; margin: 0;
}
</style>
