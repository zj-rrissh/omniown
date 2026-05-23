<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  fetchDocument,
  fetchDocuments,
  fetchStatus,
  searchDocuments,
  type DocumentDetail,
  type DocumentSummary,
  type SearchResult,
  type StatusResponse
} from './api'

type ListItem = DocumentSummary | SearchResult
type StatItem = readonly [label: string, value: number]

const status = ref<StatusResponse | null>(null)
const items = ref<ListItem[]>([])
const selected = ref<DocumentDetail | null>(null)
const query = ref('')
const mode = ref<'documents' | 'search'>('documents')
const loading = ref(false)
const error = ref<string | null>(null)

const stats = computed<StatItem[]>(() => {
  const docs = status.value?.documents
  const embeddings = status.value?.embeddings
  return [
    ['Total', docs?.total ?? 0],
    ['Public', docs?.public ?? 0],
    ['Private', docs?.private ?? 0],
    ['Indexed', docs?.indexed ?? 0],
    ['Failed', docs?.failed ?? 0],
    ['Pending', embeddings?.pending_for_current_model ?? 0]
  ]
})

const listTitle = computed(() => (mode.value === 'search' ? `Search: ${query.value}` : 'Documents'))
const isDatabaseEmpty = computed(() => (status.value?.documents.total ?? 0) === 0)

function isSearchItem(item: ListItem): item is SearchResult {
  return 'rank' in item
}

async function loadStatus(): Promise<void> {
  status.value = await fetchStatus()
}

async function loadDocuments(): Promise<void> {
  mode.value = 'documents'
  items.value = await fetchDocuments()
}

async function runSearch(): Promise<void> {
  const term = query.value.trim()
  if (!term) {
    await loadDocuments()
    return
  }

  mode.value = 'search'
  items.value = await searchDocuments(term)
}

async function selectDocument(id: number): Promise<void> {
  selected.value = await fetchDocument(id)
}

async function refresh(): Promise<void> {
  loading.value = true
  error.value = null
  try {
    await loadStatus()
    if (mode.value === 'search' && query.value.trim()) {
      await runSearch()
    } else {
      await loadDocuments()
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Request failed'
  } finally {
    loading.value = false
  }
}

async function clearSearch(): Promise<void> {
  query.value = ''
  await refresh()
}

onMounted(() => {
  void refresh()
})
</script>

<template>
  <header class="app-header">
    <div>
      <h1>OmniOwn</h1>
      <p>{{ status?.embeddings.current_model ?? 'mock-hash-384' }}</p>
    </div>
    <button type="button" class="icon-button" :disabled="loading" title="Refresh" @click="refresh">
      ↻
    </button>
  </header>

  <main class="shell">
    <aside class="sidebar">
      <section class="stats" aria-label="Status">
        <div v-for="[label, value] in stats" :key="label" class="stat">
          <span>{{ label }}</span>
          <strong>{{ value }}</strong>
        </div>
      </section>

      <form class="search" @submit.prevent="runSearch">
        <input v-model="query" type="search" placeholder="Search documents" autocomplete="off" />
        <button type="submit" class="primary">Search</button>
      </form>

      <div class="toolbar">
        <span>{{ listTitle }}</span>
        <button type="button" @click="clearSearch">All</button>
      </div>

      <div v-if="error" class="notice">{{ error }}</div>
      <div v-else-if="isDatabaseEmpty" class="notice info">
        Database is empty. Put supported files in <code>inbox/</code>, run
        <code>cargo run</code>, then refresh this page.
      </div>
      <div class="list">
        <button
          v-for="item in items"
          :key="item.id"
          type="button"
          class="row"
          :class="{ active: selected?.id === item.id }"
          @click="selectDocument(item.id)"
        >
          <span class="row-main">
            <span class="name">{{ item.filename }}</span>
            <span class="meta">{{ item.category }} · {{ item.updated_at }}</span>
            <span class="path">
              {{ isSearchItem(item) ? item.snippet ?? item.stored_path : item.stored_path }}
            </span>
          </span>
          <span class="badge" :class="{ private: item.folder_type === 'private' }">
            {{ item.folder_type }}
          </span>
        </button>
        <div v-if="!items.length && !loading" class="empty">
          {{ isDatabaseEmpty ? 'No documents imported yet.' : 'No documents found.' }}
        </div>
      </div>
    </aside>

    <section class="detail">
      <article v-if="selected" class="document">
        <div class="document-head">
          <h2>{{ selected.filename }}</h2>
          <p>{{ selected.stored_path }}</p>
          <dl>
            <div>
              <dt>Folder</dt>
              <dd>{{ selected.folder_type }}</dd>
            </div>
            <div>
              <dt>Category</dt>
              <dd>{{ selected.category }}</dd>
            </div>
            <div>
              <dt>Risk</dt>
              <dd>{{ selected.risk_level }}</dd>
            </div>
            <div>
              <dt>Updated</dt>
              <dd>{{ selected.updated_at }}</dd>
            </div>
          </dl>
        </div>
        <pre>{{ selected.content || '' }}</pre>
      </article>
      <div v-else class="empty detail-empty">Select a document to inspect extracted text.</div>
    </section>
  </main>
</template>
