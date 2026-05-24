<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { fetchStatus, type StatusResponse } from '../api'

const status = ref<StatusResponse | null>(null)
const loading = ref(true)
const error = ref<string | null>(null)

onMounted(async () => {
  try {
    status.value = await fetchStatus()
  } catch (e: any) {
    error.value = e?.message ?? '加载失败'
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="status-view">
    <h2>系统状态</h2>

    <div v-if="loading" class="loading">加载中…</div>
    <div v-else-if="error" class="error">{{ error }}</div>

    <dl v-else-if="status" class="status-grid">
      <div>
        <dt>数据库</dt>
        <dd>{{ status.database }}</dd>
      </div>
      <div>
        <dt>数据根目录</dt>
        <dd>{{ status.root }}</dd>
      </div>
      <div>
        <dt>Schema 版本</dt>
        <dd>{{ status.schema.current_version }}</dd>
      </div>
      <div>
        <dt>文档总数</dt>
        <dd>{{ status.documents.total }}</dd>
      </div>
      <div>
        <dt>公开</dt>
        <dd>{{ status.documents.public }}</dd>
      </div>
      <div>
        <dt>私有</dt>
        <dd>{{ status.documents.private }}</dd>
      </div>
      <div>
        <dt>已索引</dt>
        <dd>{{ status.documents.indexed }}</dd>
      </div>
      <div>
        <dt>失败</dt>
        <dd>{{ status.documents.failed }}</dd>
      </div>
    </dl>
  </div>
</template>

<style scoped>
.status-view {
  padding: 24px;
  color: #e0e0e0;
}

h2 {
  margin: 0 0 16px;
  font-size: 18px;
}

.loading,
.error {
  color: #888;
}

.error {
  color: #e05555;
}

.status-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.status-grid > div {
  padding: 12px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.04);
}

dt {
  font-size: 12px;
  color: #888;
  margin-bottom: 4px;
}

dd {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
}
</style>
