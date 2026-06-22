<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { Search } from '@element-plus/icons-vue'
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
const drawerVisible = ref(false)

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
  drawerVisible.value = true
}
</script>

<template>
  <div class="search-view">
    <header class="view-header" data-tauri-drag-region>
      <h1>搜索</h1>
      <el-tag v-if="status" type="info" size="small" effect="plain">
        {{ status.documents.total }} 篇
      </el-tag>
    </header>

    <div class="search-box">
      <el-input
        v-model="query"
        placeholder="输入关键词搜索…"
        :prefix-icon="Search"
        clearable
        @keyup.enter="runSearch"
      />
      <el-button
        :type="useAiSearch ? 'success' : 'default'"
        :class="{ 'is-active': useAiSearch }"
        @click="toggleSearchMode"
        title="切换搜索模式"
      >
        {{ useAiSearch ? 'AI' : '普通' }}
      </el-button>
      <el-button type="primary" :loading="loading" @click="runSearch">
        搜索
      </el-button>
    </div>

    <el-alert
      v-if="error"
      :title="error"
      type="error"
      show-icon
      :closable="false"
    />
    <el-alert
      v-else-if="isDatabaseEmpty"
      title="知识库为空。将文件放入 inbox/ 后刷新。"
      type="info"
      show-icon
      :closable="false"
    />

    <section class="search-output" aria-live="polite">
      <div class="output-head">
        <span>搜索输出</span>
        <el-tag :type="useAiSearch ? 'success' : 'info'" size="small" effect="dark">
          {{ useAiSearch ? 'AI 模式' : '普通模式' }}
        </el-tag>
      </div>
      <pre>{{ searchOutput }}</pre>
    </section>

    <el-scrollbar class="result-list">
      <div
        v-for="item in items"
        :key="item.id"
        class="result-row"
        :class="{ active: selected?.id === item.id }"
        @click="selectDocument(item.id)"
      >
        <div class="result-row__main">
          <span class="result-name">{{ item.filename }}</span>
          <span class="result-meta">
            <el-tag size="small" effect="plain" :type="item.folderType === 'private' ? 'warning' : 'default'">
              {{ item.folderType }}
            </el-tag>
            {{ item.category }}
          </span>
        </div>
        <el-tag v-if="isSearchItem(item)" type="primary" size="small" effect="dark">
          {{ item.rank.toFixed(2) }}
        </el-tag>
      </div>
      <el-empty v-if="!items.length && !loading" :description="mode === 'search' ? '无搜索结果' : '暂无文档'" />
    </el-scrollbar>

    <!-- 详情抽屉 -->
    <el-drawer
      v-model="drawerVisible"
      :title="selected?.filename ?? ''"
      size="50%"
      direction="rtl"
    >
      <template v-if="selected">
        <dl class="detail-meta">
          <div>
            <dt>路径</dt>
            <dd>{{ selected.storedPath }}</dd>
          </div>
          <div>
            <dt>类型</dt>
            <dd>
              <el-tag size="small" :type="selected.folderType === 'private' ? 'warning' : 'default'">
                {{ selected.folderType }}
              </el-tag>
              / {{ selected.category }}
            </dd>
          </div>
          <div>
            <dt>风险</dt>
            <dd>
              <el-tag size="small" :type="selected.riskLevel === 'high' ? 'danger' : selected.riskLevel === 'medium' ? 'warning' : 'info'">
                {{ selected.riskLevel }}
              </el-tag>
            </dd>
          </div>
          <div>
            <dt>更新</dt>
            <dd>{{ selected.updatedAt }}</dd>
          </div>
        </dl>
        <pre class="detail-content">{{ contentDisplay }}</pre>
      </template>
    </el-drawer>
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
  align-items: center;
  gap: 10px;
  padding: 14px 16px 0;
}
.view-header h1 {
  font-size: 16px;
  margin: 0;
}

.search-box {
  display: flex;
  gap: 8px;
  padding: 10px 16px;
}
.search-box :deep(.el-input) {
  flex: 1;
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
  align-items: center;
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
  padding: 0 8px;
}

.result-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  cursor: pointer;
  transition: background 0.15s;
  border-radius: 6px;
}
.result-row:hover,
.result-row.active {
  background: rgba(255, 255, 255, 0.06);
}
.result-row__main {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.result-name {
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.result-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: #888;
  margin-top: 2px;
}

.detail-meta {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin: 0 0 16px;
}
.detail-meta dt {
  font-size: 11px;
  color: #888;
  margin-bottom: 2px;
}
.detail-meta dd {
  font-size: 13px;
  margin: 0;
  display: flex;
  align-items: center;
  gap: 4px;
}
.detail-content {
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
