<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'

// ---- Tauri 运行时事件 ----
let appWindow: any = null
let trayShowUnlisten: (() => void) | null = null
let focusUnlisten: (() => void) | null = null
let trayTriggered = false

onMounted(async () => {
  try {
    const tauriWindow = await import('@tauri-apps/api/window')
    appWindow = tauriWindow.getCurrentWindow()

    trayShowUnlisten = await appWindow.listen('tray-show', () => {
      trayTriggered = true
      setTimeout(() => { trayTriggered = false }, 500)
    })

    focusUnlisten = await appWindow.onFocusChanged(({ payload: focused }: { payload: boolean }) => {
      if (!focused && !trayTriggered) {
        appWindow.hide()
      }
    })
  } catch {
    // 浏览器模式 — 静默跳过
  }
})

onUnmounted(() => {
  trayShowUnlisten?.()
  focusUnlisten?.()
})
</script>

<template>
  <div class="app-shell">
    <!-- 拖拽手柄 — 透明无边框窗口通过此区域拖动 -->
    <div class="drag-handle" data-tauri-drag-region></div>
    <!-- 主内容区 — Vue Router 视图 -->
    <div class="content">
      <router-view />
    </div>

    <!-- 底部导航栏 -->
    <nav class="bottom-nav">
      <router-link to="/" class="nav-item" active-class="nav-active">
        <span class="nav-icon">🔍</span>
        <span class="nav-label">搜索</span>
      </router-link>
      <router-link to="/documents" class="nav-item" active-class="nav-active">
        <span class="nav-icon">📁</span>
        <span class="nav-label">文档</span>
      </router-link>
      <router-link to="/config" class="nav-item" active-class="nav-active">
        <span class="nav-icon">⚙️</span>
        <span class="nav-label">设置</span>
      </router-link>
      <router-link to="/status" class="nav-item" active-class="nav-active">
        <span class="nav-icon">📊</span>
        <span class="nav-label">状态</span>
      </router-link>
    </nav>
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: rgba(30, 30, 40, 0.92);
  color: #e0e0e0;
}

.drag-handle {
  height: 28px;
  min-height: 28px;
  cursor: grab;
  -webkit-app-region: drag;
}

.content {
  flex: 1;
  overflow: auto;
  min-height: 0;
}

.bottom-nav {
  display: flex;
  justify-content: space-around;
  align-items: center;
  height: 52px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  background: rgba(20, 20, 30, 0.95);
}

.nav-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  text-decoration: none;
  color: #888;
  font-size: 11px;
  padding: 4px 16px;
  border-radius: 4px;
  transition: color 0.15s;
}

.nav-icon {
  font-size: 18px;
}

.nav-label {
  font-size: 11px;
}

.nav-active {
  color: #4455cc;
}
</style>
