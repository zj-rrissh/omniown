<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { FolderOpened } from '@element-plus/icons-vue'
import { fetchConfig, saveConfig, AiConfig, PathsConfig, ResolvedPathsConfig } from '../services/config.service'

const config = ref<AiConfig>({ base_url: '', model: '', api_key: '' })
const paths = ref<PathsConfig>({ root: '', library: '' })
const resolvedPaths = ref<ResolvedPathsConfig | null>(null)
const databasePath = ref('')
const saved = ref(false)
const saving = ref(false)
const error = ref<string | null>(null)

// 仅在 Tauri 环境中可用，浏览器开发模式下为 false
const isTauri = '__TAURI_INTERNALS__' in window

async function loadConfig() {
  const loaded = await fetchConfig()
  config.value = {
    base_url: loaded.base_url,
    model: loaded.model,
    api_key: loaded.api_key,
  }
  paths.value = {
    root: loaded.root,
    library: loaded.library,
  }
  resolvedPaths.value = loaded.resolved_paths ?? null
  databasePath.value = loaded.database_path ?? ''
}

onMounted(async () => {
  try {
    await loadConfig()
  } catch {
    // 浏览器开发模式或服务器暂未启动
  }
})

async function chooseDir(field: keyof PathsConfig) {
  if (!isTauri) return
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      directory: true,
      defaultPath: paths.value[field] || undefined,
    })
    if (selected) {
      paths.value[field] = selected
    }
  } catch (e) {
    console.warn('[ConfigView] 目录选择失败:', e)
  }
}

async function save() {
  saving.value = true
  error.value = null
  saved.value = false

  try {
    await saveConfig({
      ...config.value,
      ...paths.value,
    })
    await loadConfig()
    saved.value = true
    setTimeout(() => (saved.value = false), 3000)
  } catch (e: any) {
    error.value = e?.message ?? e?.toString() ?? '保存失败'
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="config-view">
    <el-form class="config-form" label-position="top" @submit.prevent="save">
      <!-- AI 配置 -->
      <h2>AI 配置</h2>
      <p class="desc">配置 LLM API 后即可使用 AI 智能搜索。</p>

      <el-form-item label="API Base URL">
        <el-input v-model="config.base_url" placeholder="https://api.deepseek.com" />
      </el-form-item>

      <el-form-item label="模型">
        <el-input v-model="config.model" placeholder="deepseek-v4-flash" />
      </el-form-item>

      <el-form-item label="API Key">
        <el-input v-model="config.api_key" type="password" show-password placeholder="sk-..." />
      </el-form-item>

      <!-- 存储路径 -->
      <h2>存储路径</h2>
      <p class="desc">选择文档库位置。数据库和配置文件由应用管理。</p>

      <el-form-item label="知识库目录（library）">
        <span class="path-row">
          <el-input v-model="paths.library" placeholder="已处理文件存储位置，默认 ./library" />
          <el-button v-if="isTauri" :icon="FolderOpened" @click="chooseDir('library')" title="选择目录" />
        </span>
        <span v-if="resolvedPaths?.library" class="resolved-path">
          实际位置：<code>{{ resolvedPaths.library }}</code>
        </span>
      </el-form-item>

      <div class="managed-paths">
        <div v-if="databasePath" class="managed-path">
          <span>数据库</span>
          <code>{{ databasePath }}</code>
        </div>
        <div v-if="resolvedPaths?.config_path" class="managed-path">
          <span>配置文件</span>
          <code>{{ resolvedPaths.config_path }}</code>
        </div>
      </div>

      <div class="form-actions">
        <el-button type="primary" native-type="submit" :loading="saving">
          {{ saving ? '保存中…' : '保存' }}
        </el-button>
        <el-alert v-if="saved" title="已保存" type="success" :closable="false" show-icon class="inline-alert" />
        <el-alert v-if="error" :title="error" type="error" :closable="false" show-icon class="inline-alert" />
      </div>
    </el-form>
  </div>
</template>

<style scoped>
.config-view {
  padding: 24px;
  color: #e0e0e0;
}

h2 {
  margin: 20px 0 4px;
  font-size: 18px;
}
h2:first-child { margin-top: 0; }

.desc {
  margin: 0 0 16px;
  color: #888;
  font-size: 12px;
  line-height: 1.5;
}

.config-form {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.path-row {
  display: flex;
  gap: 6px;
}
.path-row :deep(.el-input) {
  flex: 1;
}

.resolved-path {
  display: block;
  color: #7f8596;
  font-size: 12px;
  line-height: 1.45;
  margin-top: 4px;
}

.resolved-path code {
  color: #c9cfdd;
  font-family: ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", monospace;
  overflow-wrap: anywhere;
}

.managed-paths {
  display: grid;
  gap: 8px;
  padding: 10px 12px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.035);
}

.managed-path {
  display: grid;
  gap: 3px;
}

.managed-path span {
  color: #8d93a3;
  font-size: 12px;
}

.managed-path code {
  color: #c9cfdd;
  font-family: ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", monospace;
  font-size: 12px;
  line-height: 1.45;
  overflow-wrap: anywhere;
}

.form-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 8px;
}

.inline-alert {
  flex-shrink: 0;
}
.inline-alert :deep(.el-alert__title) {
  font-size: 13px;
}
</style>
