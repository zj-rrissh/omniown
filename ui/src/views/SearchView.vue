<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useSearchStore } from '../stores/search.store'
import { useDocumentsStore } from '../stores/documents.store'
import { fetchStatus, type StatusResponse } from '../services/status.service'
import type { DocumentSummary } from '../services/documents.service'
import type { SearchResult, SearchTrace } from '../services/search.service'
import type { SearchMode } from '../stores/search.store'

type ListItem = DocumentSummary | SearchResult

const searchStore = useSearchStore()
const docStore = useDocumentsStore()
const { query } = storeToRefs(searchStore)
const { selected } = storeToRefs(docStore)

const status = ref<StatusResponse | null>(null)
const mode = ref<'documents' | 'search'>('documents')
const useAiSearch = ref(false)

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
const searchOutput = computed(() => formatSearchOutput({
  query: searchStore.query,
  searchMode: searchStore.mode,
  loading: searchStore.loading,
  error: searchStore.error,
  trace: searchStore.trace,
  requestUrl: searchStore.requestUrl,
  resultCount: searchStore.results.length,
}))

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

function formatSearchOutput(input: {
  query: string
  searchMode: SearchMode
  loading: boolean
  error: string | null
  trace: SearchTrace | null
  requestUrl: string
  resultCount: number
}) {
  if (!input.query) return '等待搜索输入。'
  const usedAi = input.searchMode === 'ai'
  const requestUrl = input.requestUrl || input.trace?.requestUrl || '(请求尚未完成)'
  if (input.loading) {
    return usedAi
      ? `AI 搜索中...\n输入: ${input.query}\n请求: /api/search?q=${encodeURIComponent(input.query)}&limit=50&ai=true\n正在请求 LLM 选择搜索策略。`
      : `普通搜索中...\n输入: ${input.query}\n请求: /api/search?q=${encodeURIComponent(input.query)}&limit=50\n未使用 AI，直接执行 FTS5 全文搜索。`
  }
  if (input.error) {
    return [
      '搜索失败',
      `模式: ${usedAi ? 'AI 搜索' : '普通搜索'}`,
      `输入: ${input.query}`,
      `请求: ${requestUrl}`,
      `错误: ${input.error}`,
      '',
      JSON.stringify(input.trace ?? { error: input.error }, null, 2),
    ].join('\n')
  }
  if (!usedAi) {
    return [
      '普通搜索完成',
      `输入: ${input.query}`,
      `请求: ${requestUrl}`,
      'AI: 未使用',
      `结果数: ${input.resultCount}`,
    ].join('\n')
  }

  const trace = input.trace
  const lines = [
    'AI 搜索完成',
    `输入: ${input.query}`,
    `请求: ${requestUrl}`,
    `模型: ${trace?.model ?? '(未知)'}`,
    `Base URL: ${trace?.baseUrl ?? '(未知)'}`,
    `最终结果数: ${trace?.mergedResultCount ?? input.resultCount}`,
    '',
    'LLM 返回 JSON:',
    trace?.rawResponse ?? '(无返回内容)',
    '',
    '选择的策略:',
    JSON.stringify(trace?.selectedStrategies ?? [], null, 2),
    '',
    '策略搜索结果:',
    JSON.stringify(
      (trace?.strategyResults ?? []).map((item) => ({
        strategy: item.strategy,
        params: item.params,
        status: item.status,
        resultCount: item.resultCount,
        error: item.error,
        results: item.results.map((result) => ({
          id: result.id,
          filename: result.filename,
          category: result.category,
          folderType: result.folderType,
          rank: result.rank,
          snippet: result.snippet,
        })),
      })),
      null,
      2
    ),
  ]

  return lines.join('\n')
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
  const searchMode: SearchMode = useAiSearch.value ? 'ai' : 'normal'
  mode.value = 'search'
  await searchStore.search(term, searchMode)
}

function toggleSearchMode() {
  useAiSearch.value = !useAiSearch.value
  searchStore.clearOutput()
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
      <button
        type="button"
        class="mode-toggle"
        :class="{ active: useAiSearch }"
        :aria-pressed="useAiSearch"
        :title="useAiSearch ? 'AI 搜索' : '普通搜索'"
        @click="toggleSearchMode"
      >
        {{ useAiSearch ? 'AI' : '普通' }}
      </button>
      <button type="submit" class="primary" :disabled="loading">搜索</button>
    </form>

    <div v-if="error" class="notice">{{ error }}</div>
    <div v-else-if="isDatabaseEmpty" class="notice info">
      知识库为空。将文件放入 <code>inbox/</code> 后刷新。
    </div>

    <section class="search-output" aria-live="polite">
      <div class="output-head">
        <span>搜索输出</span>
        <span>{{ useAiSearch ? 'AI 模式' : '普通模式' }}</span>
      </div>
      <pre>{{ searchOutput }}</pre>
    </section>

    <div class="result-list">
      <button
        v-for="item in items"
        :key="item.id"
        type="button"
        class="result-row"
        :class="{ active: selected?.id === item.id }"
        @click="selectDocument(item.id)"
      >
        <span class="result-name">{{ item.filename }}</span>
        <span v-if="isSearchItem(item)" class="score"
          >Score: {{ item.rank.toFixed(2) }}</span
        >
        <span class="result-meta"> {{ item.category }} · {{ item.folderType }} </span>
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
        <div>
          <dt>路径</dt>
          <dd>{{ selected.storedPath }}</dd>
        </div>
        <div>
          <dt>类型</dt>
          <dd>{{ selected.folderType }} / {{ selected.category }}</dd>
        </div>
        <div>
          <dt>风险</dt>
          <dd>{{ selected.riskLevel }}</dd>
        </div>
        <div>
          <dt>更新</dt>
          <dd>{{ selected.updatedAt }}</dd>
        </div>
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
.view-header h1 {
  font-size: 16px;
  margin: 0;
}
.view-header .count {
  font-size: 12px;
  color: #888;
}

.search-box {
  display: flex;
  gap: 8px;
  padding: 10px 16px;
}
.search-box input {
  flex: 1;
  height: 34px;
  padding: 0 10px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.06);
  color: #e0e0e0;
  font-size: 13px;
}
.search-box input:focus {
  outline: none;
  border-color: #4455cc;
}
.search-box .mode-toggle {
  width: 58px;
  height: 34px;
  padding: 0;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.06);
  color: #b8bdca;
  font-size: 12px;
  cursor: pointer;
}
.search-box .mode-toggle.active {
  border-color: rgba(89, 198, 160, 0.55);
  background: rgba(89, 198, 160, 0.16);
  color: #7fe0bf;
}
.search-box button.primary {
  height: 34px;
  padding: 0 14px;
  border: none;
  border-radius: 6px;
  background: #4455cc;
  color: white;
  font-size: 13px;
  cursor: pointer;
}
.search-box button.primary:disabled {
  opacity: 0.6;
}

.notice {
  padding: 8px 16px;
  font-size: 12px;
}
.notice:not(.info) {
  color: #e05555;
}
.notice.info {
  color: #888;
}

.search-output {
  margin: 0 16px 10px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.035);
}
.output-head {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 10px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  color: #b8bdca;
  font-size: 12px;
}
.search-output pre {
  max-height: 190px;
  overflow: auto;
  padding: 10px;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: #cfd4df;
  font-size: 11px;
  line-height: 1.45;
  white-space: pre-wrap;
  margin: 0;
}

.result-list {
  flex: 1;
  overflow: auto;
  padding: 0 8px;
}

.result-row {
  display: flex;
  flex-direction: column;
  width: 100%;
  padding: 10px 12px;
  border: 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  background: none;
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.result-row:hover,
.result-row.active {
  background: rgba(255, 255, 255, 0.04);
}
.result-name {
  font-size: 13px;
}
.result-meta {
  font-size: 11px;
  color: #888;
  margin-top: 2px;
}
.score {
  font-size: 10px;
  color: #4455cc;
}

.empty {
  padding: 20px;
  text-align: center;
  color: #666;
  font-size: 13px;
}

.detail-panel {
  position: absolute;
  inset: 0;
  background: rgba(20, 20, 30, 0.98);
  display: flex;
  flex-direction: column;
  overflow: auto;
  padding: 16px;
}
.detail-head {
  display: flex;
  justify-content: space-between;
  align-items: start;
  margin-bottom: 12px;
}
.detail-head h2 {
  font-size: 16px;
  margin: 0;
}
.close-btn {
  background: none;
  border: none;
  color: #888;
  font-size: 16px;
  cursor: pointer;
}
dl {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin: 0 0 12px;
}
dt {
  font-size: 11px;
  color: #888;
}
dd {
  font-size: 12px;
  margin: 0;
}
.detail-panel pre {
  flex: 1;
  overflow: auto;
  padding: 12px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.2);
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  margin: 0;
}
</style>
