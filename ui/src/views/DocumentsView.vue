<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { Loading } from '@element-plus/icons-vue'
import { storeToRefs } from 'pinia'
import { useDocumentsStore } from '../stores/documents.store'
import { onFileChange } from '../services/events.service'

const store = useDocumentsStore()
const { filteredItems: items, selected, totalCount, folderFilter, loading, loadingMore, error, hasMore } = storeToRefs(store)

const MAX_CONTENT_LEN = 100_000
const drawerVisible = ref(false)
const sentinel = ref<HTMLElement | null>(null)
let observer: IntersectionObserver | null = null

const contentDisplay = computed(() => {
  const c = selected.value?.content
  if (!c) return '(无内容)'
  if (c.length > MAX_CONTENT_LEN) return c.slice(0, MAX_CONTENT_LEN) + '\n\n… (截断，超出部分未显示)'
  return c
})

let unsubscribe: (() => void) | null = null

onMounted(() => {
  store.loadInitial()

  // 建立 IntersectionObserver 检测滚动到底部
  observer = new IntersectionObserver(
    (entries) => {
      if (entries[0].isIntersecting) {
        store.loadMore()
      }
    },
    { rootMargin: '200px' }
  )

  // 订阅文件变更，自动刷新文档列表
  unsubscribe = onFileChange(() => {
    store.reload()
  })
})

onUnmounted(() => {
  observer?.disconnect()
  unsubscribe?.()
})

function selectDocument(id: number) {
  store.selectDocument(id)
  drawerVisible.value = true
}
</script>

<template>
  <div class="documents-view">
    <header class="view-header" data-tauri-drag-region>
      <h1>文档</h1>
      <el-tag type="info" size="small" effect="plain">{{ totalCount }} 篇</el-tag>
    </header>

    <!-- 过滤栏 -->
    <nav class="filter-bar">
      <el-button
        :type="folderFilter === 'all' ? 'primary' : 'default'"
        size="small"
        @click="store.setFilter('all')"
      >全部</el-button>
      <el-button
        :type="folderFilter === 'public' ? 'primary' : 'default'"
        size="small"
        @click="store.setFilter('public')"
      >公开</el-button>
      <el-button
        :type="folderFilter === 'private' ? 'primary' : 'default'"
        size="small"
        @click="store.setFilter('private')"
      >私有</el-button>
    </nav>

    <el-alert
      v-if="error"
      :title="error"
      type="error"
      show-icon
      :closable="false"
    />

    <!-- 文档列表（无限滚动） -->
    <div class="doc-list">
      <div
        v-for="item in items" :key="item.id"
        class="doc-row"
        :class="{ active: selected?.id === item.id }"
        @click="selectDocument(item.id)"
      >
        <div class="doc-row__main">
          <span class="doc-name">{{ item.filename }}</span>
          <span class="doc-meta">
            <el-tag size="small" effect="plain" :type="item.folderType === 'private' ? 'warning' : 'default'">
              {{ item.folderType }}
            </el-tag>
            {{ item.category }} · {{ item.updatedAt }}
          </span>
        </div>
      </div>

      <!-- 加载更多触发哨兵 -->
      <div ref="sentinel" class="scroll-sentinel">
        <el-icon v-if="loadingMore" class="is-loading">
          <Loading />
        </el-icon>
        <span v-else-if="!hasMore && items.length > 0" class="loaded-all">已加载全部</span>
      </div>

      <el-empty v-if="!items.length && !loading" description="暂无文档" />
    </div>

    <!-- 文档详情抽屉 -->
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
.documents-view {
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
.view-header h1 { font-size: 16px; margin: 0; }

.filter-bar {
  display: flex;
  gap: 6px;
  padding: 10px 16px;
}

.doc-list {
  flex: 1;
  padding: 0 12px;
}

.doc-row {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  border-bottom: 1px solid rgba(255,255,255,0.04);
  cursor: pointer;
  transition: background 0.15s;
  border-radius: 6px;
}
.doc-row:hover, .doc-row.active { background: rgba(255,255,255,0.06); }
.doc-row__main {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.doc-name {
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.doc-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: #888;
  margin-top: 2px;
}

.pagination-bar {
  display: flex;
  justify-content: center;
  padding: 10px;
  border-top: 1px solid rgba(255,255,255,0.04);
}

.scroll-sentinel {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 16px;
  min-height: 40px;
}
.scroll-sentinel .loaded-all {
  font-size: 12px;
  color: #666;
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
  border: 1px solid rgba(255,255,255,0.06);
  border-radius: 6px;
  background: rgba(0,0,0,0.2);
  font-size: 12px; line-height: 1.5; white-space: pre-wrap; margin: 0;
}
</style>
