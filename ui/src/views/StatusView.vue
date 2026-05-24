<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { fetchStatus, type StatusResponse } from '../api'

interface McpTool {
  name: string
  description: string
}

interface McpInfo {
  ready: boolean
  binary: string
  tools: McpTool[]
  claude_config: string
}

const status = ref<StatusResponse | null>(null)
const mcp = ref<McpInfo | null>(null)
const loading = ref(true)
const error = ref<string | null>(null)
const copied = ref(false)

let invoke: any = null

onMounted(async () => {
  try {
    status.value = await fetchStatus()
    // 尝试加载 Tauri MCP 状态
    const tauri = await import('@tauri-apps/api/tauri')
    invoke = tauri.invoke
    mcp.value = await invoke('mcp_info')
  } catch (e: any) {
    error.value = e?.message ?? '加载失败'
  } finally {
    loading.value = false
  }
})

async function toggleMcp() {
  if (!invoke) return
  mcp.value = await invoke('toggle_mcp')
}

async function copyConfig() {
  if (!mcp.value) return
  try {
    await navigator.clipboard.writeText(mcp.value.claude_config)
    copied.value = true
    setTimeout(() => (copied.value = false), 2000)
  } catch {
    // fallback: 选中文本
  }
}
</script>

<template>
  <div class="status-view">
    <h2>系统状态</h2>

    <div v-if="loading" class="loading">加载中…</div>
    <div v-else-if="error" class="notice">{{ error }}</div>

    <template v-else>
      <!-- 数据库状态 -->
      <dl v-if="status" class="status-grid">
        <div>
          <dt>数据库</dt>
          <dd class="path-text">{{ status.database }}</dd>
        </div>
        <div>
          <dt>数据根目录</dt>
          <dd class="path-text">{{ status.root }}</dd>
        </div>
        <div>
          <dt>Schema 版本</dt>
          <dd>{{ status.schema.current_version }}</dd>
        </div>
        <div>
          <dt>待迁移</dt>
          <dd>{{ status.schema.pending_migrations }}</dd>
        </div>
        <div>
          <dt>文档总数</dt>
          <dd>{{ status.documents.total }}</dd>
        </div>
        <div>
          <dt>公开 / 私有</dt>
          <dd>{{ status.documents.public }} / {{ status.documents.private }}</dd>
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

      <!-- MCP 状态 -->
      <section v-if="mcp" class="mcp-section">
        <h3>MCP Server</h3>
        <div class="mcp-status-row">
          <span class="mcp-badge" :class="{ on: mcp.ready }">
            {{ mcp.ready ? '🟢 已启用' : '⚫ 未启用' }}
          </span>
          <button v-if="invoke" class="toggle-btn" @click="toggleMcp">
            {{ mcp.ready ? '关闭' : '开启' }}
          </button>
        </div>

        <div class="mcp-tools">
          <h4>可用工具 ({{ mcp.tools.length }})</h4>
          <ul>
            <li v-for="tool in mcp.tools" :key="tool.name">
              <code>{{ tool.name }}</code>
              <span>{{ tool.description }}</span>
            </li>
          </ul>
        </div>

        <div class="mcp-config">
          <h4>AI 客户端配置</h4>
          <p class="hint">将以下内容粘贴到 Claude Desktop / Cursor 的 MCP 配置中：</p>
          <pre class="config-snippet">{{ mcp.claude_config }}</pre>
          <button class="copy-btn" @click="copyConfig">
            {{ copied ? '✅ 已复制' : '📋 复制配置' }}
          </button>
        </div>
      </section>
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
h4 { margin: 16px 0 8px; font-size: 14px; color: #aaa; }

.loading { color: #888; }
.notice { color: #e05555; font-size: 13px; }

.path-text {
  font-size: 12px !important;
  word-break: break-all;
  opacity: 0.7;
}

/* 数据库状态 */
.status-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
.status-grid > div {
  padding: 12px;
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 6px;
  background: rgba(255,255,255,0.04);
}
dt { font-size: 12px; color: #888; margin-bottom: 4px; }
dd { font-size: 16px; font-weight: 600; margin: 0; }

/* MCP */
.mcp-section {
  margin-top: 8px;
}

.mcp-status-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.mcp-badge {
  font-size: 14px;
  padding: 4px 12px;
  border-radius: 999px;
  border: 1px solid rgba(255,255,255,0.1);
  background: rgba(255,255,255,0.04);
}
.mcp-badge.on {
  border-color: #4ec94e33;
  background: #4ec94e18;
  color: #4ec94e;
}

.toggle-btn {
  height: 30px;
  padding: 0 14px;
  border: 1px solid rgba(255,255,255,0.15);
  border-radius: 6px;
  background: rgba(255,255,255,0.06);
  color: #ccc;
  font-size: 12px;
  cursor: pointer;
}
.toggle-btn:hover { background: rgba(255,255,255,0.1); }

.mcp-tools ul {
  list-style: none;
  padding: 0;
  margin: 0;
}
.mcp-tools li {
  display: flex;
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px solid rgba(255,255,255,0.04);
  font-size: 12px;
}
.mcp-tools code {
  color: #4455cc;
  white-space: nowrap;
  min-width: 120px;
}
.mcp-tools span {
  color: #888;
}

.mcp-config .hint {
  font-size: 12px;
  color: #888;
  margin: 0 0 8px;
}

.config-snippet {
  padding: 12px;
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 6px;
  background: rgba(0,0,0,0.3);
  color: #aaa;
  font-size: 11px;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0 0 8px;
}

.copy-btn {
  height: 32px;
  padding: 0 16px;
  border: 1px solid #4455cc44;
  border-radius: 6px;
  background: #4455cc22;
  color: #8899ee;
  font-size: 12px;
  cursor: pointer;
}
.copy-btn:hover { background: #4455cc33; }
</style>
