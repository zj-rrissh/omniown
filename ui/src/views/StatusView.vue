<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { fetchStatus, type StatusResponse } from '../services/status.service'

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

    <el-skeleton v-if="loading" :rows="4" animated />
    <el-alert v-else-if="error" :title="error" type="error" show-icon :closable="false" />

    <template v-else-if="status">
      <!-- 核心状态卡片 -->
      <div class="status-grid">
        <el-card shadow="never">
          <template #header><span>数据库</span></template>
          <code class="path-text">{{ status.database }}</code>
        </el-card>
        <el-card shadow="never">
          <template #header><span>数据根目录</span></template>
          <code class="path-text">{{ status.root }}</code>
        </el-card>
        <el-card shadow="never">
          <template #header><span>Schema 版本</span></template>
          <strong>{{ status.schema.current_version }}</strong>
        </el-card>
        <el-card shadow="never">
          <template #header><span>待迁移</span></template>
          <strong>{{ status.schema.pending_migrations }}</strong>
        </el-card>
      </div>

      <!-- 文档统计 -->
      <h3>文档统计</h3>
      <div class="status-grid">
        <el-card shadow="never">
          <template #header><span>文档总数</span></template>
          <strong>{{ status.documents.total }}</strong>
        </el-card>
        <el-card shadow="never">
          <template #header><span>公开 / 私有</span></template>
          <div class="tag-row">
            <el-tag type="info" effect="plain">{{ status.documents.public }} 公开</el-tag>
            <el-tag type="warning" effect="plain">{{ status.documents.private }} 私有</el-tag>
          </div>
        </el-card>
        <el-card shadow="never">
          <template #header><span>已索引</span></template>
          <el-tag type="success" effect="dark">{{ status.documents.indexed }}</el-tag>
        </el-card>
        <el-card shadow="never">
          <template #header><span>失败</span></template>
          <el-tag :type="status.documents.failed > 0 ? 'danger' : 'info'" effect="dark">{{ status.documents.failed }}</el-tag>
        </el-card>
      </div>
    </template>
  </div>
</template>

<style scoped>
.status-view {
  padding: 24px;
  color: #e0e0e0;
}

h2 { margin: 0 0 16px; font-size: 18px; }
h3 { margin: 24px 0 12px; font-size: 16px; }

.path-text {
  font-size: 12px;
  word-break: break-all;
  opacity: 0.7;
}

.status-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.status-grid strong {
  font-size: 22px;
  font-weight: 650;
}

.tag-row {
  display: flex;
  gap: 8px;
}
</style>
