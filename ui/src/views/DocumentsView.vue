<script setup lang="ts">
import { onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useDocumentsStore } from '../stores/documents.store'

const store = useDocumentsStore()
const { pagedItems: items, selected, totalCount, page, totalPages, folderFilter, loading, error } = storeToRefs(store)

onMounted(() => store.loadDocuments())
</script>

<template>
  <div class="documents-view">
    <header class="view-header" data-tauri-drag-region>
      <h1>文档</h1>
      <span class="count">{{ totalCount }} 篇</span>
    </header>

    <!-- 过滤栏 -->
    <nav class="filter-bar">
      <button :class="{ active: folderFilter === 'all' }" @click="store.setFilter('all')">全部</button>
      <button :class="{ active: folderFilter === 'public' }" @click="store.setFilter('public')">公开</button>
      <button :class="{ active: folderFilter === 'private' }" @click="store.setFilter('private')">私有</button>
    </nav>

    <div v-if="error" class="notice">{{ error }}</div>

    <!-- 文档列表 -->
    <div class="doc-list">
      <button
        v-for="item in items" :key="item.id"
        type="button"
        class="doc-row"
        :class="{ active: selected?.id === item.id }"
        @click="store.selectDocument(item.id)"
      >
        <span class="doc-name">{{ item.filename }}</span>
        <span class="doc-meta">
          <span class="badge" :class="item.folder_type">{{ item.folder_type }}</span>
          {{ item.category }} · {{ item.updated_at }}
        </span>
      </button>
      <div v-if="!items.length && !loading" class="empty">暂无文档</div>
    </div>

    <!-- 分页 -->
    <div class="pagination" v-if="totalPages > 1">
      <button :disabled="page <= 1" @click="store.prevPage">← 上一页</button>
      <span>{{ page }} / {{ totalPages }}</span>
      <button :disabled="page >= totalPages" @click="store.nextPage">下一页 →</button>
    </div>

    <!-- 文档详情 -->
    <section v-if="selected" class="detail-panel">
      <div class="detail-head">
        <h2>{{ selected.filename }}</h2>
        <button class="close-btn" @click="store.selected = null">✕</button>
      </div>
      <dl>
        <div><dt>路径</dt><dd>{{ selected.stored_path }}</dd></div>
        <div><dt>类型</dt><dd>{{ selected.folder_type }} / {{ selected.category }}</dd></div>
        <div><dt>风险</dt><dd>{{ selected.risk_level }}</dd></div>
        <div><dt>更新</dt><dd>{{ selected.updated_at }}</dd></div>
      </dl>
      <pre>{{ selected?.content ? (selected.content.length > 100000 ? selected.content.slice(0, 100000) + '\n\n… (截断)' : selected.content) : '(无内容)' }}</pre>
    </section>
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
  align-items: baseline;
  gap: 10px;
  padding: 14px 16px 0;
}
.view-header h1 { font-size: 16px; margin: 0; }
.view-header .count { font-size: 12px; color: #888; }

.filter-bar {
  display: flex;
  gap: 6px;
  padding: 10px 16px;
}
.filter-bar button {
  height: 28px;
  padding: 0 12px;
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 6px;
  background: rgba(255,255,255,0.04);
  color: #888;
  font-size: 12px;
  cursor: pointer;
}
.filter-bar button.active {
  border-color: #4455cc44;
  background: #4455cc22;
  color: #aabbee;
}

.notice { padding: 8px 16px; color: #e05555; font-size: 12px; }

.doc-list {
  flex: 1;
  overflow: auto;
  padding: 0 12px;
}

.doc-row {
  display: flex;
  flex-direction: column;
  width: 100%;
  padding: 10px 12px;
  border: 0;
  border-bottom: 1px solid rgba(255,255,255,0.04);
  background: none;
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.doc-row:hover, .doc-row.active { background: rgba(255,255,255,0.04); }
.doc-name { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.doc-meta { font-size: 11px; color: #888; margin-top: 2px; display: flex; gap: 6px; align-items: center; }

.badge {
  display: inline-block;
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 999px;
  border: 1px solid rgba(255,255,255,0.1);
  color: #aaa;
}
.badge.private { border-color: #d9b99d44; color: #d9b99d; }

.empty { padding: 20px; text-align: center; color: #666; font-size: 13px; }

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 10px;
  border-top: 1px solid rgba(255,255,255,0.04);
}
.pagination button {
  height: 28px;
  padding: 0 12px;
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 6px;
  background: rgba(255,255,255,0.04);
  color: #aaa;
  font-size: 12px;
  cursor: pointer;
}
.pagination button:disabled { opacity: 0.3; cursor: default; }
.pagination span { font-size: 12px; color: #888; }

.detail-panel {
  position: absolute;
  inset: 0;
  background: rgba(20, 20, 30, 0.98);
  display: flex;
  flex-direction: column;
  overflow: auto;
  padding: 16px;
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
