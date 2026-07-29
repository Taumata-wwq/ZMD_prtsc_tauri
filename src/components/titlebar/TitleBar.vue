<template>
  <header class="titlebar" data-tauri-drag-region>
    <!-- 左侧：图标 + 应用名 + 版本号 -->
    <div class="tb-title" data-tauri-drag-region>
      <span class="tb-icon" data-tauri-drag-region>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
          stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z" />
          <circle cx="12" cy="13" r="4" />
        </svg>
      </span>
      <span class="tb-name">终末地截图工具</span>
      <span class="tb-version">v3.0.0</span>
    </div>

    <!-- 中间：占满剩余空间的可拖拽区 -->
    <div class="tb-drag-spacer" data-tauri-drag-region="" />

    <!-- 右侧：主题切换 + 窗口控制按钮（主题移到标题栏） -->
    <div class="tb-actions">
      <button
        class="tb-btn tb-theme"
        type="button"
        :title="theme === 'dark' ? t('titlebar.themeToggleLight') : t('titlebar.themeToggleDark')"
        :aria-label="theme === 'dark' ? t('titlebar.themeToggleLight') : t('titlebar.themeToggleDark')"
        @click="toggleTheme"
      >
        <!-- 深色主题显示"月亮"图标，浅色主题显示"太阳"图标 -->
        <svg v-if="theme === 'dark'" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
          stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
        <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
          stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="5" />
          <line x1="12" y1="1" x2="12" y2="3" />
          <line x1="12" y1="21" x2="12" y2="23" />
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
          <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
          <line x1="1" y1="12" x2="3" y2="12" />
          <line x1="21" y1="12" x2="23" y2="12" />
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
          <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
        </svg>
      </button>
      <button
        class="tb-btn"
        :class="{ active: alwaysOnTop }"
        type="button"
        :title="t('titlebar.pin')"
        :aria-pressed="alwaysOnTop"
        @click="toggleAlwaysOnTop"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
          stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 17v5" />
          <path
            d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"
          />
        </svg>
      </button>
      <button
        class="tb-btn"
        type="button"
        :title="t('titlebar.minimize')"
        @click="onMinimize"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor"
          stroke-width="2" stroke-linecap="round">
          <path d="M5 12h14" />
        </svg>
      </button>
      <button
        class="tb-btn tb-close"
        type="button"
        :title="t('titlebar.close')"
        @click="onClose"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor"
          stroke-width="2" stroke-linecap="round">
          <path d="M18 6L6 18M6 6l12 12" />
        </svg>
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import {
  setAlwaysOnTop,
  minimizeWindow,
  closeWindow,
  getAlwaysOnTop,
} from '../../api/window'
import { useSettingsStore } from '@/stores/settings.store'
import { useI18n } from '@/composables/useI18n'
import type { AppSettings } from '@/types'

const settingsStore = useSettingsStore()
const { t } = useI18n()

/** 窗口是否处于置顶状态 */
const alwaysOnTop = ref(false)

/** 主题 */
const theme = ref<'dark' | 'light'>('dark')

onMounted(async () => {
  try {
    alwaysOnTop.value = await getAlwaysOnTop()
  } catch (err) {
    console.warn('[TitleBar] 读取置顶状态失败:', err)
  }
  // 同步当前主题（settingsStore 已在 App.vue 加载）
  if (settingsStore.settings) {
    theme.value = settingsStore.settings.theme
    document.documentElement.dataset.theme = settingsStore.settings.theme
  }
})

async function toggleAlwaysOnTop() {
  const next = !alwaysOnTop.value
  try {
    await setAlwaysOnTop(next)
    alwaysOnTop.value = next
  } catch (err) {
    console.error('[TitleBar] 切换置顶失败:', err)
  }
}

/** 切换主题：立即应用到 DOM 并持久化到 settings */
async function toggleTheme() {
  const next: AppSettings['theme'] = theme.value === 'dark' ? 'light' : 'dark'
  theme.value = next
  document.documentElement.dataset.theme = next
  try {
    await settingsStore.update('theme', next)
  } catch (err) {
    console.error('[TitleBar] 持久化主题失败:', err)
  }
}

async function onMinimize() {
  try {
    await minimizeWindow()
  } catch (err) {
    console.error('[TitleBar] 最小化失败:', err)
  }
}

async function onClose() {
  try {
    await closeWindow()
  } catch (err) {
    console.error('[TitleBar] 关闭失败:', err)
  }
}
</script>

<style scoped>
.titlebar {
  display: flex;
  align-items: center;
  height: 36px;
  padding: 0 8px 0 12px;
  background: var(--titlebar-bg);
  border-bottom: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  user-select: none;
  -webkit-user-select: none;
  flex-shrink: 0;
}

.tb-title {
  display: flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
  flex-shrink: 0;
}

.tb-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--accent);
}

.tb-name {
  font-size: 12px;
  color: var(--text-secondary);
  font-weight: 500;
  letter-spacing: 0.2px;
}

.tb-version {
  font-size: 11px;
  color: var(--text-muted);
  font-weight: 400;
}

.tb-drag-spacer {
  flex: 1;
  height: 100%;
}

.tb-actions {
  display: flex;
  align-items: center;
  gap: 1px;
  flex-shrink: 0;
}

.tb-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 28px;
  padding: 0;
  border: none;
  border-radius: 0;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}

.tb-btn:hover {
  background: var(--bg-tertiary);
  color: var(--text-primary);
}

.tb-btn.active {
  color: var(--accent);
  background: var(--accent-light);
}

.tb-btn.active:hover {
  background: var(--accent-light);
  color: var(--accent);
}

.tb-close:hover {
  background: #e81123;
  color: #ffffff;
}
</style>
