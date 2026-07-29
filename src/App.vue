<template>
  <div class="app-root" :data-theme="theme">
    <!-- 全局错误条：监听三个 store 的 lastError，3 秒后自动消失 -->
    <div
      v-if="captureStore.lastError || configStore.lastError || settingsStore.lastError"
      class="global-error-bar"
    >
      {{ captureStore.lastError || configStore.lastError || settingsStore.lastError }}
    </div>

    <!-- 顶部 TitleBar：拖拽 + 窗口控制 -->
    <TitleBar />

    <!-- 主体：左侧 48px 图标导航 + 右侧视图 -->
    <div class="app-body">
      <aside class="sidebar">
        <button
          v-for="item in navItems"
          :key="item.key"
          class="nav-btn"
          :class="{ active: currentView === item.key }"
          type="button"
          :title="t(item.titleKey)"
          :aria-current="currentView === item.key"
          @click="currentView = item.key"
        >
          <span class="nav-icon" v-html="item.iconSvg" />
        </button>
      </aside>

      <main class="main-content">
        <!-- 使用 keep-alive 缓存视图，避免切换时重新加载截图调整区域 -->
        <keep-alive>
          <component :is="currentComponent" />
        </keep-alive>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import TitleBar from './components/titlebar/TitleBar.vue'
import CaptureView from './views/CaptureView.vue'
import HistoryView from './views/HistoryView.vue'
import DataManageView from './views/DataManageView.vue'
import SettingsView from './views/SettingsView.vue'
import { useSettingsStore } from '@/stores/settings.store'
import { useConfigStore } from '@/stores/config.store'
import { useCaptureStore } from '@/stores/capture.store'
import { useI18n } from '@/composables/useI18n'
import type { CaptureLog } from '@/types'

// 数据管理位于历史下方
type ViewKey = 'capture' | 'history' | 'data' | 'settings'

const theme = ref<'dark' | 'light'>('dark')
const currentView = ref<ViewKey>('capture')

const settingsStore = useSettingsStore()
const configStore = useConfigStore()
const captureStore = useCaptureStore()
const { t } = useI18n()

const navItems: { key: ViewKey; titleKey: string; iconSvg: string }[] = [
  {
    key: 'capture',
    titleKey: 'nav.capture',
    iconSvg:
      '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"/><circle cx="12" cy="13" r="4"/></svg>',
  },
  {
    key: 'history',
    titleKey: 'nav.history',
    iconSvg:
      '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3v5h5"/><path d="M3.05 13A9 9 0 1 0 6 5.3L3 8"/><path d="M12 7v5l4 2"/></svg>',
  },
  // 数据管理（位于历史下方）
  {
    key: 'data',
    titleKey: 'nav.data',
    iconSvg:
      '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14a9 3 0 0 0 18 0V5"/><path d="M3 12a9 3 0 0 0 18 0"/></svg>',
  },
  {
    key: 'settings',
    titleKey: 'nav.settings',
    iconSvg:
      '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>',
  },
]

const currentComponent = computed(() => {
  switch (currentView.value) {
    case 'capture':
      return CaptureView
    case 'history':
      return HistoryView
    case 'data':
      return DataManageView
    case 'settings':
      return SettingsView
  }
})

/** 热键事件监听 unlisten 句柄集合，组件卸载时统一释放 */
const unlistenFns: UnlistenFn[] = []

function pushLog(level: CaptureLog['level'], message: string) {
  const log: CaptureLog = {
    level,
    message,
    timestamp: new Date().toLocaleString('zh-CN', { hour12: false }),
  }
  captureStore.logs.push(log)
}

onMounted(async () => {
  // 启动日志（与原 Python 项目的 startup_msgs 对齐）
  pushLog('info', '[ZMD] 终末地截图工具启动中...')

  // 1. 加载设置（theme / output_format / last_region / last_scroll_mode 等）
  try {
    await settingsStore.load()
  } catch (e) {
    console.error('[ZMD] 加载设置失败:', e)
    pushLog('error', `[ZMD] 加载设置失败: ${e}`)
  }

  // 2. 应用主题（立即同步到 DOM，避免闪烁）
  if (settingsStore.settings) {
    theme.value = settingsStore.settings.theme
    document.documentElement.dataset.theme = settingsStore.settings.theme
  }
  pushLog('info', '[ZMD] 数据库已初始化')

  // 3. 加载配置（区域列表 + 滚动模式列表）
  try {
    await configStore.load()
  } catch (e) {
    console.error('[ZMD] 加载配置失败:', e)
    pushLog('error', `[ZMD] 加载配置失败: ${e}`)
  }

  // 4. 恢复上次选择（last_region / last_scroll_mode 覆盖 configStore 默认值）
  if (settingsStore.settings) {
    if (settingsStore.settings.last_region) {
      configStore.currentRegionName = settingsStore.settings.last_region
    }
    if (settingsStore.settings.last_scroll_mode) {
      configStore.currentScrollModeName = settingsStore.settings.last_scroll_mode
    }
  }

  pushLog(
    'info',
    `[ZMD] 已加载 ${configStore.regions.length} 个区域配置`,
  )
  pushLog(
    'info',
    `[ZMD] 已加载 ${configStore.scrollModes.length} 个滚动模式`,
  )

  // 5. 初始化截图 store（监听 capture:progress / capture:log / capture:status / capture:preview-ready）
  try {
    await captureStore.init()
  } catch (e) {
    console.error('[ZMD] 截图 store 初始化失败:', e)
    pushLog('error', `[ZMD] 截图 store 初始化失败: ${e}`)
  }

  // 6. 监听全局热键事件（开始/停止合并为 F3）
  try {
    unlistenFns.push(
      await listen<string>('hotkey', (e) => {
        switch (e.payload) {
          case 'F3': {
            // 合并按钮：根据 isRunning 状态切换开始/停止
            if (captureStore.isRunning) {
              captureStore.stop().catch((err) => pushLog('error', `[ZMD] 停止截图失败: ${err}`))
              break
            }
            // 开始截图：需要当前区域与滚动模式已选定
            const region = configStore.currentRegionName
            if (!region) {
              pushLog('warn', '[ZMD] 未选择区域，无法开始截图')
              break
            }
            const scrollMode = configStore.currentScrollModeName
            if (!scrollMode) {
              pushLog('warn', '[ZMD] 未选择滚动模式，无法开始截图')
              break
            }
            // 仅"自定义"区域使用 last_rows/last_cols 覆盖网格（用户可编辑行列数）
            // 预设区域（武陵/谷地/大地图）由 Rust 端按 region_config 的 grid_rows/grid_cols 决定
            const isCustomGrid = region === '自定义'
            const rows = isCustomGrid ? settingsStore.settings?.last_rows : undefined
            const cols = isCustomGrid ? settingsStore.settings?.last_cols : undefined
            captureStore
              .start(region, scrollMode, rows, cols)
              .catch((err) => pushLog('error', `[ZMD] 启动截图失败: ${err}`))
            break
          }
          default:
            console.warn('[ZMD] 未知热键:', e.payload)
        }
      }),
    )
    pushLog('info', '[ZMD] 全局热键已注册: F3=开始/停止')
  } catch (e) {
    console.error('[ZMD] 热键事件监听失败:', e)
    pushLog('error', `[ZMD] 热键事件监听失败: ${e}`)
  }

  pushLog('info', '[ZMD] 应用就绪')
  console.log('[ZMD] 应用初始化完成')

  // 所有初始化完成后显示窗口（避免启动时白屏）
  try {
    await getCurrentWindow().show()
  } catch (e) {
    console.warn('[ZMD] 窗口显示失败:', e)
  }
})

onUnmounted(() => {
  // 释放所有热键事件监听
  unlistenFns.forEach((fn) => {
    try {
      fn()
    } catch {
      // 忽略已释放的监听器
    }
  })
  // 释放 captureStore 的事件监听（capture:* 系列）
  captureStore.dispose()
})
</script>

<style scoped>
.app-root {
  display: flex;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
  background: var(--bg-primary);
  overflow: hidden;
}

/* 全局错误条：红色背景、白色文字、固定在顶部、无圆角 */
.global-error-bar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 9999;
  padding: 8px 16px;
  background: #d32f2f;
  color: #ffffff;
  font-size: 14px;
  font-weight: 500;
  border-radius: 0;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
  text-align: center;
  pointer-events: none;
  user-select: none;
}

.app-body {
  flex: 1;
  display: flex;
  min-height: 0;
}

.sidebar {
  width: 48px;
  background: var(--sidebar-bg);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 8px 0;
  flex-shrink: 0;
}

.nav-btn {
  position: relative;
  width: 48px;
  height: 48px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s ease, color 0.15s ease;
}

.nav-btn:hover {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.nav-btn.active {
  color: var(--accent);
  background: var(--accent-light);
}

/* 左侧 2px 强调色边框 */
.nav-btn.active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 2px;
  background: var(--accent);
}

.nav-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
}

.main-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  min-width: 0;
}
</style>
