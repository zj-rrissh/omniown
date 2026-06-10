<script setup lang="ts">
import { onMounted, ref } from 'vue'
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
    <form class="config-form" @submit.prevent="save">
      <!-- AI 配置 -->
      <h2>AI 配置</h2>
      <p class="desc">配置 LLM API 后即可使用 AI 智能搜索。</p>

      <label>
        <span>API Base URL</span>
        <input v-model="config.base_url" type="url" placeholder="https://api.openai.com/v1" />
      </label>

      <label>
        <span>模型</span>
        <input v-model="config.model" type="text" placeholder="gpt-4o-mini" />
      </label>

      <label>
        <span>API Key</span>
        <input v-model="config.api_key" type="password" placeholder="sk-..." />
      </label>

      <!-- 存储路径 -->
      <h2>存储路径</h2>
      <p class="desc">选择文档库位置。数据库和配置文件由应用管理。</p>

      <label>
        <span>知识库目录（library）</span>
        <span class="path-row">
          <input v-model="paths.library" type="text" placeholder="已处理文件存储位置，默认 ./library" />
          <button v-if="isTauri" type="button" class="browse-btn" @click="chooseDir('library')" title="选择目录">📁</button>
        </span>
        <span v-if="resolvedPaths?.library" class="resolved-path">
          实际位置：<code>{{ resolvedPaths.library }}</code>
        </span>
      </label>

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
        <button type="submit" class="primary" :disabled="saving">
          {{ saving ? '保存中…' : '保存' }}
        </button>
        <span v-if="saved" class="saved-msg">✅ 已保存</span>
        <span v-if="error" class="error-msg">{{ error }}</span>
      </div>
    </form>
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
  gap: 14px;
}

label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 13px;
  color: #aaa;
}

.path-row {
  display: flex;
  gap: 6px;
}

.path-row input {
  flex: 1;
}

.resolved-path {
  color: #7f8596;
  font-size: 12px;
  line-height: 1.45;
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

.browse-btn {
  flex-shrink: 0;
  width: 36px;
  height: 36px;
  padding: 0;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.08);
  color: #aaa;
  font-size: 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s, border-color 0.15s;
}

.browse-btn:hover {
  background: rgba(255, 255, 255, 0.14);
  border-color: rgba(255, 255, 255, 0.25);
}

input {
  height: 36px;
  padding: 0 10px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.06);
  color: #e0e0e0;
  font-size: 13px;
}

input:focus {
  outline: none;
  border-color: #4455cc;
}

input::placeholder {
  color: #555;
}

.form-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 8px;
}

button.primary {
  height: 36px;
  padding: 0 20px;
  border: none;
  border-radius: 6px;
  background: #4455cc;
  color: white;
  font-size: 13px;
  cursor: pointer;
}

button.primary:disabled {
  opacity: 0.6;
  cursor: wait;
}

.saved-msg {
  color: #4ec94e;
  font-size: 13px;
}

.error-msg {
  color: #e05555;
  font-size: 13px;
}
</style>
