<template>
  <div class="capture-view">
    <!-- 左侧配置面板：宽度持久化到 localStorage -->
    <aside
      class="capture-left"
      :style="{ width: leftWidth + 'px', flexBasis: leftWidth + 'px' }"
    >
      <ConfigPanel />
      <div class="capture-hint">
        <div class="hint-title">{{ t('capture.hint.title') }}</div>
        <pre class="hint-text">{{ t('capture.hint.items') }}</pre>
      </div>
    </aside>

    <div
      class="splitter"
      @mousedown="onSplitterMouseDown"
    >
      <div class="splitter-handle" />
    </div>

    <section class="capture-right">
      <header class="capture-toolbar">
        <!-- 截图完成后才出现的按钮（保存裁剪 + 清除选区） -->
        <div class="toolbar-left">
          <button
            v-if="previewUrl"
            type="button"
            class="action-btn btn-primary"
            :disabled="cropperBusy"
            :title="t('capture.saveCropped')"
            @click="onSaveCropped"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
              stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M6 2v14a2 2 0 0 0 2 2h14" />
              <path d="M18 22V8a2 2 0 0 0-2-2H2" />
            </svg>
            <span>{{ t('capture.saveCropped') }}</span>
          </button>
          <button
            v-if="previewUrl"
            type="button"
            class="action-btn"
            :disabled="cropperBusy"
            :title="t('capture.clearSelection')"
            @click="onClearSelection"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
              stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 6h18" />
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
              <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            </svg>
            <span>{{ t('capture.clearSelection') }}</span>
          </button>
        </div>

        <div class="toolbar-right">
          <button
            type="button"
            class="action-btn"
            :class="captureStore.isRunning ? 'btn-danger' : 'btn-primary'"
            :title="captureStore.isRunning ? t('capture.stop') + ' (F3)' : t('capture.start') + ' (F3)'"
            @click="onToggle"
          >
            <svg v-if="!captureStore.isRunning" width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M8 5v14l11-7z" />
            </svg>
            <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <rect x="6" y="6" width="12" height="12" rx="1" />
            </svg>
            <span>{{ captureStore.isRunning ? t('capture.stop') : t('capture.start') }}</span>
          </button>
        </div>
      </header>

      <div class="capture-preview">
        <PreviewCropper
          v-if="previewUrl"
          ref="cropperRef"
          :image-url="previewUrl"
          :session-id="captureStore.currentSessionId"
          @exported="onExported"
        />
        <div v-else-if="captureStore.isRunning" class="preview-processing">
          <div class="spinner" />
          <p class="processing-text">{{ captureStore.progress?.current ?? 0 }} / {{ captureStore.progress?.total ?? 0 }}</p>
        </div>
        <div v-else-if="captureStore.isProcessing" class="preview-processing">
          <div class="spinner" />
          <p class="processing-text">{{ t('capture.processing') }}</p>
        </div>
        <div v-else class="preview-placeholder">
          <svg width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2" />
            <circle cx="8.5" cy="8.5" r="1.5" />
            <polyline points="21 15 16 10 5 21" />
          </svg>
          <p class="placeholder-text">{{ t('capture.waiting') }}</p>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
// CaptureView：预览区集成 PreviewCropper（拖拽/缩放/四角调整），保存直接写入 screenshot_folder
import { ref, computed } from 'vue'
import ConfigPanel from '@/components/capture/ConfigPanel.vue'
import PreviewCropper from '@/components/capture/PreviewCropper.vue'
import { useCaptureStore } from '@/stores/capture.store'
import { useConfigStore } from '@/stores/config.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useHistoryStore } from '@/stores/history.store'
import { useI18n } from '@/composables/useI18n'
import { useSplitter } from '@/composables/useSplitter'

const captureStore = useCaptureStore()
const configStore = useConfigStore()
const settingsStore = useSettingsStore()
const historyStore = useHistoryStore()
const { t } = useI18n()

const cropperRef = ref<InstanceType<typeof PreviewCropper> | null>(null)

/** PreviewCropper 正在导出时禁用按钮 */
const cropperBusy = computed<boolean>(() => cropperRef.value?.isExporting ?? false)

// store 中 previewImageUrl 已用 convertFileSrc 转为可访问 URL
const previewUrl = computed(() => captureStore.previewImageUrl)

// 开始/停止合并为单一按钮（F3 热键）
async function onToggle() {
  if (captureStore.isRunning) {
    try {
      await captureStore.stop()
    } catch (e) {
      console.error('[CaptureView] stop 失败:', e)
    }
    return
  }
  const region = configStore.currentRegionName
  if (!region) {
    console.warn('[CaptureView] 未选择区域，无法开始截图')
    return
  }
  const scrollMode = configStore.currentScrollModeName
  if (!scrollMode) {
    console.warn('[CaptureView] 未选择滚动模式，无法开始截图')
    return
  }
  // 自定义类别与大地图自定义模式需要显式传 rows/cols，其余由 Rust 端按区域配置解析
  const isCustom = configStore.currentRegion?.category === '自定义'
  const isLargeMapCustom = !!settingsStore.settings?.last_large_map_custom
  const needManualGrid = isCustom || isLargeMapCustom
  const rows = needManualGrid ? (settingsStore.settings?.last_rows ?? 2) : undefined
  const cols = needManualGrid ? (settingsStore.settings?.last_cols ?? 2) : undefined
  try {
    await captureStore.start(region, scrollMode, rows, cols)
  } catch (e) {
    console.error('[CaptureView] start 失败:', e)
  }
}

/** 保存成功后刷新历史记录，使历史页路径与按钮状态同步 */
function onExported(_payload: { path: string; cropped: boolean }) {
  void historyStore.load().catch((e) => {
    console.error('[CaptureView] 刷新历史记录失败:', e)
  })
}

function onSaveCropped() {
  cropperRef.value?.saveCropped()
}

function onClearSelection() {
  cropperRef.value?.clearSelection()
}

const { width: leftWidth, onMouseDown: onSplitterMouseDown } = useSplitter({
  storageKey: 'capture_left_width',
  defaultWidth: 200,
  minWidth: 150,
  maxWidth: 400,
  containerSelector: '.capture-view',
})
</script>

<style scoped>
.capture-view {
  display: flex;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--bg-primary);
}

/* 处理中加载指示器 */
.preview-processing {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.processing-text {
  color: var(--text-muted);
  font-size: 12px;
}

/* 左侧配置面板：宽度由 leftWidth 控制 */
.capture-left {
  flex-shrink: 0;
  border-right: 1px solid var(--border);
  background: var(--bg-secondary);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* 让 ConfigPanel 在剩余空间内可滚动 */
.capture-left > :first-child {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.capture-hint {
  flex-shrink: 0;
  border-top: 1px solid var(--border);
  padding: 8px 12px;
  background: var(--bg-tertiary);
  font-family: inherit;
}

.hint-title {
  font-size: 11px;
  color: var(--text-secondary);
  margin-bottom: 4px;
  font-weight: 600;
}

.hint-text {
  margin: 0;
  font-family: inherit;
  font-size: 11px;
  line-height: 1.6;
  color: var(--text-muted);
  white-space: pre-wrap;
  word-break: break-word;
  user-select: text;
}

/* 可拖拽分割线 */
.splitter {
  width: 6px;
  flex-shrink: 0;
  cursor: col-resize;
  background: var(--border);
  position: relative;
  transition: background 0.15s ease;
}

.splitter:hover,
.splitter:active {
  background: var(--accent);
}

.splitter-handle {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 2px;
  height: 32px;
  background: var(--text-muted);
  border-radius: 1px;
  opacity: 0.5;
  pointer-events: none;
}

.splitter:hover .splitter-handle,
.splitter:active .splitter-handle {
  background: #ffffff;
  opacity: 0.8;
}

.capture-right {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.capture-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  flex-wrap: wrap;
}

.toolbar-left,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

/* 通用按钮 */
.action-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 12px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--bg-tertiary);
  color: var(--text-primary);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease, opacity 0.15s ease;
  white-space: nowrap;
}

.action-btn:hover:not(:disabled) {
  background: var(--btn-hover-bg);
  border-color: var(--accent);
}

.action-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.action-btn svg {
  flex-shrink: 0;
}

/* 主按钮：开始 */
.action-btn.btn-primary {
  background: var(--accent);
  color: #ffffff;
  border-color: var(--accent);
}

.action-btn.btn-primary:hover:not(:disabled) {
  background: var(--accent-hover);
  border-color: var(--accent-hover);
  color: #ffffff;
}

/* 危险按钮：停止 */
.action-btn.btn-danger {
  background: #e81123;
  color: #ffffff;
  border-color: #e81123;
}

.action-btn.btn-danger:hover:not(:disabled) {
  background: #c50f1f;
  border-color: #c50f1f;
  color: #ffffff;
}

.capture-preview {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #1a1a1a;
  background-image:
    linear-gradient(45deg, #2a2a2a 25%, transparent 25%),
    linear-gradient(-45deg, #2a2a2a 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, #2a2a2a 75%),
    linear-gradient(-45deg, transparent 75%, #2a2a2a 75%);
  background-size: 16px 16px;
  background-position: 0 0, 0 8px, 8px -8px, -8px 0;
  overflow: hidden;
  min-height: 0;
  padding: 12px;
}

.preview-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-muted);
}

.placeholder-text {
  font-size: 14px;
  color: var(--text-secondary);
  margin: 0;
}
</style>