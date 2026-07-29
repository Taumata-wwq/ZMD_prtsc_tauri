<template>
  <div class="settings-view">
    <div v-if="settingsStore.loading" class="loading">{{ t('common.loading') }}</div>
    <div v-else class="settings-scroll">
      <!-- 延迟设置组 -->
      <section class="setting-group">
        <h2 class="group-title">{{ t('settings.delayGroup') }}</h2>
        <div class="setting-item" :title="t('settings.stabilizeDelayHint')">
          <span class="si-label">{{ t('delay.stabilize') }}</span>
          <div class="si-right">
            <span class="si-value">{{ formatMs(settingsStore.settings?.stabilize_delay ?? 0.1) }}</span>
            <input
              type="range"
              class="si-slider"
              min="0.03"
              max="0.5"
              step="0.005"
              :value="settingsStore.settings?.stabilize_delay ?? 0.1"
              @input="onNumberChange('stabilize_delay', ($event.target as HTMLInputElement).value, 0.03, 0.5)"
            />
          </div>
        </div>
        <div class="setting-item" :title="t('settings.screenshotDelayHint')">
          <span class="si-label">{{ t('delay.screenshot') }}</span>
          <div class="si-right">
            <span class="si-value">{{ formatMs(settingsStore.settings?.screenshot_delay ?? 0.1) }}</span>
            <input
              type="range"
              class="si-slider"
              min="0.03"
              max="0.5"
              step="0.005"
              :value="settingsStore.settings?.screenshot_delay ?? 0.1"
              @input="onNumberChange('screenshot_delay', ($event.target as HTMLInputElement).value, 0.03, 0.5)"
            />
          </div>
        </div>
        <div class="setting-item" :title="t('settings.dragDurationHint')">
          <span class="si-label">{{ t('delay.dragDuration') }}</span>
          <div class="si-right">
            <span class="si-value">{{ formatMs(settingsStore.settings?.drag_duration ?? 0.05) }}</span>
            <input
              type="range"
              class="si-slider"
              min="0.03"
              max="0.5"
              step="0.005"
              :value="settingsStore.settings?.drag_duration ?? 0.05"
              @input="onNumberChange('drag_duration', ($event.target as HTMLInputElement).value, 0.03, 0.5)"
            />
          </div>
        </div>
      </section>

      <!-- 拖拽设置组 -->
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
        <!-- 拖拽时距离边界的距离设置 -->
        <div class="setting-item" :title="t('settings.dragMarginBottomHint')">
          <span class="si-label">{{ t('delay.dragMarginBottom') }}</span>
          <div class="si-right">
            <span class="si-value">{{ settingsStore.settings?.drag_margin_bottom ?? 10 }}px</span>
            <input
              type="range"
              class="si-slider"
              min="0"
              max="100"
              step="1"
              :value="settingsStore.settings?.drag_margin_bottom ?? 10"
              @input="onNumberChange('drag_margin_bottom', ($event.target as HTMLInputElement).value, 0, 100, true)"
            />
          </div>
        </div>
        <div class="setting-item" :title="t('settings.dragMarginLeftHint')">
          <span class="si-label">{{ t('delay.dragMarginLeft') }}</span>
          <div class="si-right">
            <span class="si-value">{{ settingsStore.settings?.drag_margin_left ?? 10 }}px</span>
            <input
              type="range"
              class="si-slider"
              min="0"
              max="100"
              step="1"
              :value="settingsStore.settings?.drag_margin_left ?? 10"
              @input="onNumberChange('drag_margin_left', ($event.target as HTMLInputElement).value, 0, 100, true)"
            />
          </div>
        </div>
      </section>

      <!-- 输出目录组 -->
      <section class="setting-group">
        <h2 class="group-title">{{ t('settings.outputGroup') }}</h2>
        <div class="setting-item setting-item-col" :title="t('settings.outputFolderHint')">
          <div class="si-row">
            <span class="si-label">{{ t('settings.outputFolder') }}</span>
            <div class="si-right">
              <button class="browse-btn" :disabled="browsingFolder" @click="onBrowseFolder">
                {{ browsingFolder ? '...' : t('capture.selectDir') }}
              </button>
            </div>
          </div>
          <input
            type="text"
            class="si-input"
            :value="settingsStore.settings?.output_folder ?? ''"
            placeholder="screenshots"
            @change="onFolderChange(($event.target as HTMLInputElement).value)"
          />
        </div>
        <!-- 自定义导出文件名格式 -->
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

      <!-- 关于组 -->
      <section class="setting-group">
        <h2 class="group-title">{{ t('settings.about') }}</h2>
        <div class="setting-item setting-item-col">
          <div class="about-grid">
            <span class="about-key">{{ t('about.appName') }}</span>
            <span class="about-val">终末地截图工具</span>
            <span class="about-key">{{ t('about.version') }}</span>
            <span class="about-val">v1.0.0</span>
            <span class="about-key">{{ t('about.techStack') }}</span>
            <span class="about-val">Tauri 2 + Vue 3 + Rust</span>
            <span class="about-key">{{ t('about.license') }}</span>
            <span class="about-val">MIT</span>
          </div>
        </div>
      </section>

      <!-- 语言组 -->
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

      <!-- 日志组 -->
      <section class="setting-group setting-group-grow">
        <h2 class="group-title">{{ t('capture.logs') }}</h2>
        <div class="setting-item setting-item-col setting-item-grow">
          <LogPanel class="log-panel-wrap" />
        </div>
      </section>
    </div>

    <div v-if="settingsStore.error" class="error-bar">
      {{ t('error.settingsLoad') }}：{{ settingsStore.error }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useSettingsStore } from '@/stores/settings.store'
import { useConfigStore } from '@/stores/config.store'
import { useI18n } from '@/composables/useI18n'
import LogPanel from '@/components/capture/LogPanel.vue'
import type { AppSettingKey } from '@/types'

const settingsStore = useSettingsStore()
const configStore = useConfigStore()
const { t } = useI18n()

const browsingFolder = ref<boolean>(false)

/** 格式化毫秒显示 */
function formatMs(sec: number): string {
  return `${Math.round(sec * 1000)}ms`
}

async function onFolderChange(value: string) {
  try {
    await settingsStore.update('output_folder', value)
  } catch (e) {
    console.error('[SettingsView] 更新输出目录失败:', e)
  }
}

async function onBrowseFolder() {
  browsingFolder.value = true
  try {
    const selected = await openDialog({ directory: true, multiple: false })
    if (typeof selected === 'string' && selected.length > 0) {
      await onFolderChange(selected)
    }
  } catch (e) {
    console.error('[SettingsView] 选择目录失败:', e)
  } finally {
    browsingFolder.value = false
  }
}

/**
 * 数值类型设置项变更（统一处理整数与浮点数）
 *
 * @param key 设置键名
 * @param raw 输入字符串
 * @param min 最小值
 * @param max 最大值
 * @param isInt 是否为整数（true 用 parseInt，false 用 parseFloat），默认 false
 */
async function onNumberChange(
  key: AppSettingKey,
  raw: string,
  min: number,
  max: number,
  isInt = false,
) {
  const num = isInt ? parseInt(raw, 10) : parseFloat(raw)
  if (Number.isNaN(num)) return
  const clamped = Math.min(max, Math.max(min, num))
  try {
    await settingsStore.update(key, String(clamped))
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
    await settingsStore.update('minimize_on_capture', checked ? 'true' : 'false')
  } catch (e) {
    console.error('[SettingsView] 切换最小化选项失败:', e)
  }
}

onMounted(async () => {
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

/* 卡片分组 */
.setting-group {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.setting-group-grow {
  flex: 1;
  min-height: 200px;
}

.group-title {
  font-size: 10px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.8px;
  margin-bottom: 6px;
  padding: 0 4px;
}

/* 设置项卡片 */
.setting-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  gap: 12px;
}

.setting-item + .setting-item {
  border-top: none;
}

.setting-item-col {
  flex-direction: column;
  align-items: stretch;
  gap: 6px;
}

.setting-item-grow {
  flex: 1;
  min-height: 0;
}

.si-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.si-label {
  font-size: 12px;
  color: var(--text-primary);
  flex-shrink: 0;
}

.si-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.si-value {
  font-size: 11px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
  min-width: 36px;
  text-align: right;
}

/* slider 拖拽条 */
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

.si-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--accent);
  cursor: pointer;
  border: 1px solid var(--bg-primary);
}

.si-slider::-moz-range-thumb {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--accent);
  cursor: pointer;
  border: 1px solid var(--bg-primary);
}

/* select */
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

.si-select:focus {
  border-color: var(--accent);
}

/* input */
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

.si-input:focus {
  border-color: var(--accent);
}

.si-hint {
  font-size: 10px;
  color: var(--text-muted);
  line-height: 1.4;
  word-break: break-all;
}

/* 浏览按钮 */
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

.browse-btn:hover:not(:disabled) {
  background: var(--btn-hover-bg);
  border-color: var(--accent);
}

.browse-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* 关于网格 */
.about-grid {
  display: grid;
  grid-template-columns: 100px 1fr;
  gap: 4px 12px;
  font-size: 11px;
  width: 100%;
}

.about-key {
  color: var(--text-secondary);
}

.about-val {
  color: var(--text-primary);
}

/* 日志面板 */
.log-panel-wrap {
  flex: 1;
  border: 1px solid var(--border);
  border-radius: 3px;
  overflow: hidden;
  min-height: 120px;
}

/* 开关样式 */
.switch {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 20px;
  flex-shrink: 0;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

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

.switch input:checked + .switch-slider {
  background: var(--accent);
  border-color: var(--accent);
}

.switch input:checked + .switch-slider::before {
  transform: translateX(16px);
  background: #ffffff;
}

/* 错误条 */
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
</style>
