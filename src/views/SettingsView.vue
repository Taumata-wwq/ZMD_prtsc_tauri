<template>
  <div class="settings-view">
    <div v-if="settingsStore.loading" class="loading">{{ t('common.loading') }}</div>
    <div v-else class="settings-scroll">
      <section class="setting-group">
        <h2 class="group-title">{{ t('settings.delayGroup') }}</h2>
        <div
          v-for="item in delaySliders"
          :key="item.key"
          class="setting-item"
          :title="t(item.hintKey)"
        >
          <span class="si-label">{{ t(item.labelKey) }}</span>
          <div class="si-right">
            <span class="si-value">{{ formatMs(settingsStore.settings?.[item.key] ?? item.fallback) }}</span>
            <input
              type="range"
              class="si-slider"
              min="0.03"
              max="0.5"
              step="0.005"
              :value="settingsStore.settings?.[item.key] ?? item.fallback"
              @input="onNumberChange(item.key, ($event.target as HTMLInputElement).value, 0.03, 0.5)"
            />
          </div>
        </div>
      </section>

      <section class="setting-group">
        <h2 class="group-title">{{ t('settings.dragGroup') }}</h2>
        <div class="setting-item" :title="t('settings.minimizeOnCaptureHint')">
          <span class="si-label">{{ t('settings.minimizeOnCapture') }}</span>
          <div class="si-right">
            <label class="switch">
              <input
                type="checkbox"
                :checked="settingsStore.settings?.minimize_on_capture ?? true"
                @change="onMinimizeOnCaptureChange(($event.target as HTMLInputElement).checked)"
              />
              <span class="switch-slider" />
            </label>
          </div>
        </div>
        <div
          v-for="item in dragMargins"
          :key="item.key"
          class="setting-item"
          :title="t(item.hintKey)"
        >
          <span class="si-label">{{ t(item.labelKey) }}</span>
          <div class="si-right">
            <span class="si-value">{{ settingsStore.settings?.[item.key] ?? item.fallback }}px</span>
            <input
              type="range"
              class="si-slider"
              min="0"
              max="100"
              step="1"
              :value="settingsStore.settings?.[item.key] ?? item.fallback"
              @input="onNumberChange(item.key, ($event.target as HTMLInputElement).value, 0, 100, true)"
            />
          </div>
        </div>
      </section>

      <section class="setting-group">
        <h2 class="group-title">{{ t('settings.outputGroup') }}</h2>
        <div
          v-for="item in folderInputs"
          :key="item.key"
          class="setting-item setting-item-col"
          :title="t(item.hintKey)"
        >
          <div class="si-row">
            <span class="si-label">{{ t(item.labelKey) }}</span>
            <div class="si-right">
              <button class="browse-btn" :disabled="browsingFolder === item.target" @click="onBrowseFolder(item.target)">
                {{ browsingFolder === item.target ? '...' : t('capture.selectDir') }}
              </button>
            </div>
          </div>
          <input
            type="text"
            class="si-input"
            :value="settingsStore.settings?.[item.key] ?? ''"
            :placeholder="item.placeholder"
            @change="onFolderChange(item.key, ($event.target as HTMLInputElement).value)"
          />
        </div>
        <div class="setting-item setting-item-col" :title="t('settings.filenamePatternHint')">
          <div class="si-row">
            <span class="si-label">{{ t('settings.filenamePattern') }}</span>
          </div>
          <input
            type="text"
            class="si-input"
            :value="settingsStore.settings?.filename_pattern ?? '{region}_{timestamp}_{scrollMode}'"
            placeholder="{region}_{timestamp}_{scrollMode}"
            @change="onFilenamePatternChange(($event.target as HTMLInputElement).value)"
          />
          <span class="si-hint">{{ t('settings.filenamePatternHint') }}</span>
        </div>
      </section>

      <section class="setting-group">
        <h2 class="group-title">{{ t('settings.about') }}</h2>
        <div class="setting-item setting-item-col">
          <div class="about-grid">
            <template v-for="item in aboutItems" :key="item.keyKey">
              <span class="about-key">{{ t(item.keyKey) }}</span>
              <span class="about-val">{{ t(item.valKey) }}</span>
            </template>
          </div>
        </div>
      </section>

      <section class="setting-group">
        <h2 class="group-title">{{ t('settings.dataGroup') }}</h2>
        <div class="setting-item setting-item-col">
          <div class="si-row">
            <span class="si-label">{{ t('settings.resetData') }}</span>
            <div class="si-right">
              <button
                class="browse-btn btn-danger"
                :disabled="isConfirming"
                @click="showResetDialog"
              >
                {{ isConfirming ? '...' : t('settings.resetDataBtn') }}
              </button>
            </div>
          </div>
          <span class="si-hint">{{ t('settings.resetDataHint') }}</span>
        </div>
      </section>

      <section class="setting-group">
        <h2 class="group-title">{{ t('settings.language') }}</h2>
        <div class="setting-item" :title="t('settings.languageHint')">
          <span class="si-label">{{ t('settings.language') }}</span>
          <div class="si-right">
            <select
              class="si-select"
              :value="settingsStore.settings?.language ?? 'zh'"
              @change="onLanguageChange"
            >
              <option value="zh">{{ t('settings.language.zh') }}</option>
              <option value="en">{{ t('settings.language.en') }}</option>
            </select>
          </div>
        </div>
      </section>

      <section class="setting-group setting-group-grow">
        <h2 class="group-title">{{ t('capture.logs') }}</h2>
        <div class="setting-item setting-item-col setting-item-grow">
          <LogPanel class="log-panel-wrap" />
        </div>
      </section>
    </div>

    <div v-if="settingsStore.lastError" class="error-bar">
      {{ t('error.settingsLoad') }}：{{ settingsStore.lastError }}
    </div>

    <div v-if="isOpen" class="modal-overlay" @click.self="closeResetDialog">
      <div class="modal-dialog">
        <h3 class="modal-title">{{ t('settings.resetConfirmTitle') }}</h3>
        <p class="modal-message">{{ t('settings.resetConfirmMessage') }}</p>
        <label class="checkbox-label">
          <input type="checkbox" v-model="includeHistory" />
          <span>{{ t('settings.resetIncludeHistory') }}</span>
        </label>
        <div class="modal-actions">
          <button type="button" class="browse-btn" :disabled="isConfirming" @click="closeResetDialog">{{ t('common.cancel') }}</button>
          <button type="button" class="browse-btn btn-danger" :disabled="isConfirming" @click="confirmReset">
            {{ isConfirming ? '...' : t('common.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { appDataDir } from '@tauri-apps/api/path'
import { useSettingsStore } from '@/stores/settings.store'
import { useConfigStore } from '@/stores/config.store'
import { useI18n } from '@/composables/useI18n'
import { useConfirmDialog } from '@/composables/useConfirmDialog'
import { api } from '@/api'
import LogPanel from '@/components/capture/LogPanel.vue'
import type { AppSettingKey, AppSettings } from '@/types'

const settingsStore = useSettingsStore()
const configStore = useConfigStore()
const { t } = useI18n()

/** 当前正在浏览的文件夹 key */
const browsingFolder = ref<'original' | 'screenshot' | 'thumbnail' | null>(null)

/** appDataDir 缓存（用于计算 placeholder 默认路径） */
const dataDir = ref<string>('')

const defaultOriginalPlaceholder = computed(() => (dataDir.value ? `${dataDir.value}/originals` : 'originals'))
const defaultScreenshotPlaceholder = computed(() => (dataDir.value ? `${dataDir.value}/screenshots` : 'screenshots'))
const defaultThumbnailPlaceholder = computed(() => (dataDir.value ? `${dataDir.value}/thumbnails` : 'thumbnails'))

// 延迟类滑块配置（统一 0.03-0.5 范围，0.005 步长）
const delaySliders: { key: 'stabilize_delay' | 'screenshot_delay' | 'drag_duration'; labelKey: string; hintKey: string; fallback: number }[] = [
  { key: 'stabilize_delay', labelKey: 'delay.stabilize', hintKey: 'settings.stabilizeDelayHint', fallback: 0.1 },
  { key: 'screenshot_delay', labelKey: 'delay.screenshot', hintKey: 'settings.screenshotDelayHint', fallback: 0.1 },
  { key: 'drag_duration', labelKey: 'delay.dragDuration', hintKey: 'settings.dragDurationHint', fallback: 0.07 },
]

// 拖拽边界滑块配置（整数像素值，0-100 范围）
const dragMargins: { key: 'drag_margin_bottom' | 'drag_margin_left'; labelKey: string; hintKey: string; fallback: number }[] = [
  { key: 'drag_margin_bottom', labelKey: 'delay.dragMarginBottom', hintKey: 'settings.dragMarginBottomHint', fallback: 10 },
  { key: 'drag_margin_left', labelKey: 'delay.dragMarginLeft', hintKey: 'settings.dragMarginLeftHint', fallback: 10 },
]

const folderInputs = computed(() => [
  {
    key: 'original_folder' as const,
    target: 'original' as const,
    labelKey: 'settings.originalFolder',
    hintKey: 'settings.originalFolderHint',
    placeholder: defaultOriginalPlaceholder.value,
  },
  {
    key: 'screenshot_folder' as const,
    target: 'screenshot' as const,
    labelKey: 'settings.screenshotFolder',
    hintKey: 'settings.screenshotFolderHint',
    placeholder: defaultScreenshotPlaceholder.value,
  },
  {
    key: 'thumbnail_folder' as const,
    target: 'thumbnail' as const,
    labelKey: 'settings.thumbnailFolder',
    hintKey: 'settings.thumbnailFolderHint',
    placeholder: defaultThumbnailPlaceholder.value,
  },
])

const aboutItems: { keyKey: string; valKey: string }[] = [
  { keyKey: 'about.appName', valKey: 'about.appValue' },
  { keyKey: 'about.version', valKey: 'about.versionValue' },
  { keyKey: 'about.techStack', valKey: 'about.techStackValue' },
  { keyKey: 'about.license', valKey: 'about.licenseValue' },
]

const { isOpen, isConfirming, open, close } = useConfirmDialog()
const includeHistory = ref(false)

function showResetDialog() {
  includeHistory.value = false
  open()
}

function closeResetDialog() {
  close()
}

async function confirmReset() {
  isConfirming.value = true
  try {
    await api.resetData(includeHistory.value)
    // 重置后重新加载设置和区域配置，并恢复上次选择的区域
    await settingsStore.load()
    await configStore.load()
    if (settingsStore.settings?.last_region) {
      configStore.currentRegionName = settingsStore.settings.last_region
    }
    await configStore.refreshCurrentRegion()
    close()
  } catch (e) {
    console.error('[SettingsView] 重置数据失败:', e)
  } finally {
    isConfirming.value = false
  }
}

function formatMs(sec: number): string {
  return `${Math.round(sec * 1000)}ms`
}

type FolderKey = 'original_folder' | 'screenshot_folder' | 'thumbnail_folder'

async function onFolderChange(key: FolderKey, value: string) {
  try {
    await settingsStore.update(key, value)
  } catch (e) {
    console.error(`[SettingsView] 更新 ${key} 失败:`, e)
  }
}

async function onBrowseFolder(target: 'original' | 'screenshot' | 'thumbnail') {
  browsingFolder.value = target
  try {
    const selected = await openDialog({ directory: true, multiple: false })
    if (typeof selected === 'string' && selected.length > 0) {
      const key: FolderKey = `${target}_folder` as FolderKey
      await onFolderChange(key, selected)
    }
  } catch (e) {
    console.error('[SettingsView] 选择目录失败:', e)
  } finally {
    browsingFolder.value = null
  }
}

async function onNumberChange<K extends AppSettingKey>(
  key: K,
  raw: string,
  min: number,
  max: number,
  isInt = false,
) {
  const num = isInt ? parseInt(raw, 10) : parseFloat(raw)
  if (Number.isNaN(num)) return
  const clamped = Math.min(max, Math.max(min, num))
  try {
    await settingsStore.update(key, clamped as AppSettings[K])
  } catch (e) {
    console.error(`[SettingsView] 更新 ${key} 失败:`, e)
  }
}

async function onFilenamePatternChange(value: string) {
  try {
    await settingsStore.update('filename_pattern', value)
  } catch (e) {
    console.error('[SettingsView] 更新文件名格式失败:', e)
  }
}

async function onLanguageChange(event: Event) {
  const target = event.target as HTMLSelectElement
  const lang = target.value as 'zh' | 'en'
  try {
    await settingsStore.update('language', lang)
  } catch (e) {
    console.error('[SettingsView] 切换语言失败:', e)
  }
}

async function onMinimizeOnCaptureChange(checked: boolean) {
  try {
    await settingsStore.update('minimize_on_capture', checked)
  } catch (e) {
    console.error('[SettingsView] 切换最小化选项失败:', e)
  }
}

onMounted(async () => {
  try {
    dataDir.value = await appDataDir()
  } catch (e) {
    console.error('[SettingsView] 获取 appDataDir 失败:', e)
  }
  try {
    await settingsStore.load()
  } catch (e) {
    console.error('[SettingsView] settings.load 失败:', e)
  }
  try {
    await configStore.load()
  } catch (e) {
    console.error('[SettingsView] config.load 失败:', e)
  }
})
</script>

<style scoped>
.settings-view {
  position: relative;
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-primary);
  overflow: hidden;
}

.loading {
  padding: 32px;
  text-align: center;
  color: var(--text-muted);
}

.settings-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 700px;
  width: 100%;
  margin: 0 auto;
}

.setting-group { display: flex; flex-direction: column; gap: 1px; }
.setting-group-grow { flex: 1; min-height: 200px; }

.group-title {
  font-size: 10px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.8px;
  margin-bottom: 6px;
  padding: 0 4px;
}

.setting-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  gap: 12px;
}

.setting-item + .setting-item { border-top: none; }
.setting-item-col { flex-direction: column; align-items: stretch; gap: 6px; }
.setting-item-grow { flex: 1; min-height: 0; }
.si-row { display: flex; align-items: center; justify-content: space-between; width: 100%; }
.si-label { font-size: 12px; color: var(--text-primary); flex-shrink: 0; }
.si-right { display: flex; align-items: center; gap: 8px; }

.si-value {
  font-size: 11px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
  min-width: 36px;
  text-align: right;
}

.si-slider {
  width: 160px;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: var(--input-border);
  border-radius: 2px;
  outline: none;
  cursor: pointer;
}

.si-slider::-webkit-slider-thumb,
.si-slider::-moz-range-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--accent);
  cursor: pointer;
  border: 1px solid var(--bg-primary);
}

.si-select {
  height: 26px;
  padding: 0 8px;
  border: 1px solid var(--input-border);
  border-radius: 3px;
  background: var(--input-bg);
  color: var(--text-primary);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  outline: none;
  min-width: 140px;
}

.si-select:focus { border-color: var(--accent); }

.si-input {
  width: 100%;
  height: 26px;
  padding: 0 8px;
  border: 1px solid var(--input-border);
  border-radius: 3px;
  background: var(--input-bg);
  color: var(--text-primary);
  font-size: 11px;
  font-family: inherit;
  outline: none;
  box-sizing: border-box;
}

.si-input:focus { border-color: var(--accent); }
.si-hint { font-size: 10px; color: var(--text-muted); line-height: 1.4; word-break: break-all; }

.browse-btn {
  height: 26px;
  padding: 0 10px;
  border: 1px solid var(--border);
  border-radius: 3px;
  background: var(--bg-tertiary);
  color: var(--text-primary);
  font-size: 11px;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
  white-space: nowrap;
}

.browse-btn:hover:not(:disabled) { background: var(--btn-hover-bg); border-color: var(--accent); }
.browse-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.about-grid {
  display: grid;
  grid-template-columns: 100px 1fr;
  gap: 4px 12px;
  font-size: 11px;
  width: 100%;
}

.about-key { color: var(--text-secondary); }
.about-val { color: var(--text-primary); }

.log-panel-wrap {
  flex: 1;
  border: 1px solid var(--border);
  border-radius: 3px;
  overflow: hidden;
  min-height: 120px;
}

.switch {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 20px;
  flex-shrink: 0;
}

.switch input { opacity: 0; width: 0; height: 0; }

.switch-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 10px;
  transition: 0.2s;
}

.switch-slider::before {
  position: absolute;
  content: "";
  height: 14px;
  width: 14px;
  left: 2px;
  bottom: 2px;
  background: var(--text-secondary);
  border-radius: 50%;
  transition: 0.2s;
}

.switch input:checked + .switch-slider { background: var(--accent); border-color: var(--accent); }
.switch input:checked + .switch-slider::before { transform: translateX(16px); background: #ffffff; }

.error-bar {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  padding: 6px 12px;
  background: #4d2929;
  color: #ffcccc;
  font-size: 11px;
  text-align: center;
}

.browse-btn.btn-danger { background: #e81123; color: #ffffff; border-color: #e81123; }
.browse-btn.btn-danger:hover:not(:disabled) { background: #c50f1f; border-color: #c50f1f; }

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-dialog {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 20px;
  min-width: 360px;
  max-width: 460px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

.modal-title { margin: 0 0 12px 0; font-size: 14px; font-weight: 600; color: var(--text-primary); }
.modal-message { margin: 0 0 16px 0; font-size: 13px; color: var(--text-secondary); line-height: 1.5; }

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-primary);
  cursor: pointer;
  margin-bottom: 18px;
}

.checkbox-label input[type="checkbox"] { margin: 0; cursor: pointer; }
.modal-actions { display: flex; justify-content: flex-end; gap: 8px; }
</style>
