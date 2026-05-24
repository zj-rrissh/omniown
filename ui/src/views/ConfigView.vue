<script setup lang="ts">
import { onMounted, ref } from 'vue'

interface AiConfig {
  base_url: string
  model: string
  api_key: string
}

const config = ref<AiConfig>({ base_url: '', model: '', api_key: '' })
const saved = ref(false)
const saving = ref(false)
const error = ref<string | null>(null)

let invoke: any = null

onMounted(async () => {
  try {
    const tauri = await import('@tauri-apps/api/tauri')
    invoke = tauri.invoke
    config.value = await invoke('read_config')
  } catch {
    // 浏览器开发模式 — 使用空默认值
  }
})

async function save() {
  saving.value = true
  error.value = null
  saved.value = false

  try {
    if (invoke) {
      await invoke('write_config', { aiConfig: config.value })
    }
    saved.value = true
    setTimeout(() => (saved.value = false), 3000)
  } catch (e: any) {
    error.value = e?.toString() ?? '保存失败'
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="config-view">
    <h2>AI 配置</h2>
    <p class="desc">配置 LLM API 后即可使用 AI 智能搜索。</p>

    <form class="config-form" @submit.prevent="save">
      <label>
        <span>API Base URL</span>
        <input
          v-model="config.base_url"
          type="url"
          placeholder="https://api.openai.com/v1"
        />
      </label>

      <label>
        <span>模型</span>
        <input
          v-model="config.model"
          type="text"
          placeholder="gpt-4o-mini"
        />
      </label>

      <label>
        <span>API Key</span>
        <input
          v-model="config.api_key"
          type="password"
          placeholder="sk-..."
        />
      </label>

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
  margin: 0 0 4px;
  font-size: 18px;
}

.desc {
  margin: 0 0 20px;
  color: #888;
  font-size: 13px;
}

.config-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 13px;
  color: #aaa;
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
  margin-top: 4px;
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
