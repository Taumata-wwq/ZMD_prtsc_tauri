<template>
  <div class="capture-view">
    <!-- 左侧配置面板：宽度可拖拽调节（持久化到 localStorage） -->
    <aside
      class="capture-left"
      :style="{ width: leftWidth + 'px', flexBasis: leftWidth + 'px' }"
    >
      <ConfigPanel />
      <!-- 左下方使用说明 -->
      <div class="capture-hint">
        <div class="hint-title">{{ t('capture.hint.title') }}</div>
        <pre class="hint-text">{{ t('capture.hint.items') }}</pre>
      </div>
    </aside>

    <!-- 中部分割线：可拖拽调节左右占比 -->
    <div
      class="splitter"
      @mousedown="onSplitterMouseDown"
    >
      <div class="splitter-handle" />
    </div>

    <!-- 右侧主体：上中下三段 -->
    <section class="capture-right">
      <!-- 上部：截图按钮 + 截图信息 + 开始/停止按钮 -->
      <header class="capture-toolbar">
        <!-- 左侧：截图完成后才出现的三个按钮 -->
        <div class="toolbar-left">
          <button
            v-if="previewUrl"
            type="button"
            class="action-btn btn-primary"
            :disabled="cropperBusy"
            :title="t('capture.exportCropped')"
            @click="onExportCropped"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
              stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M6 2v14a2 2 0 0 0 2 2h14" />
              <path d="M18 22V8a2 2 0 0 0-2-2H2" />
            </svg>
            <span>{{ t('capture.exportCropped') }}</span>
          </button>
          <button
            v-if="previewUrl"
            type="button"
            class="action-btn btn-primary"
            :disabled="cropperBusy"
            :title="t('capture.exportOriginal')"
            @click="onExportOriginal"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
              stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
            <span>{{ t('capture.exportOriginal') }}</span>
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

        <!-- 右侧：开始/停止按钮 -->
        <div class="toolbar-right">
          <button
            type="button"
            class="action-btn"
            :class="captureStore.isRunning ? 'btn-danger' : 'btn-primary'"
            :title="captureStore.isRunning ? t('capture.stop') + ' (F3)' : t('capture.start') + ' (F3)'"
            @click="onToggle"
          >
            <!-- 开始：三角图标 -->
            <svg v-if="!captureStore.isRunning" width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M8 5v14l11-7z" />
            </svg>
            <!-- 停止：方块图标 -->
            <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <rect x="6" y="6" width="12" height="12" rx="1" />
            </svg>
            <span>{{ captureStore.isRunning ? t('capture.stop') : t('capture.start') }}</span>
          </button>
        </div>
      </header>

      <!-- 中部：预览区域 -->
      <!-- 取消实时显示，截图过程中仅显示进度，全部完成后统一拼接 -->
      <div class="capture-preview">
        <PreviewCropper
          v-if="previewUrl"
          ref="cropperRef"
          :image-url="previewUrl"
          :source="captureStore.currentSource"
          :session-id="captureStore.currentSessionId"
          @exported="onExported"
        />
        <!-- 截图进行中显示进度信息 -->
        <div v-else-if="captureStore.isRunning" class="preview-processing">
          <div class="spinner" />
          <p class="processing-text">{{ captureStore.progress?.current ?? 0 }} / {{ captureStore.progress?.total ?? 0 }}</p>
        </div>
        <!-- 处理中显示加载指示器 -->
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
// =============================================================================
// CaptureView
//
// 主界面：
//   - 预览区域直接集成裁剪交互（PreviewCropper）
//   - PreviewCropper 支持拖拽移动选区、四角调整大小、滚轮同步缩放
//   - 保存时不弹对话框，直接保存到 output_folder
// =============================================================================
import { ref, computed, onMounted } from 'vue'
import ConfigPanel from '@/components/capture/ConfigPanel.vue'
import PreviewCropper from '@/components/capture/PreviewCropper.vue'
import { useCaptureStore } from '@/stores/capture.store'
import { useConfigStore } from '@/stores/config.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useI18n } from '@/composables/useI18n'
import { useSplitter } from '@/composables/useSplitter'

const captureStore = useCaptureStore()
const configStore = useConfigStore()
const settingsStore = useSettingsStore()
const { t } = useI18n()

// ---------------------------------------------------------------------------
// PreviewCropper 引用（用于调用暴露的 exportCropped/exportOriginal/clearSelection）
// ---------------------------------------------------------------------------
const cropperRef = ref<InstanceType<typeof PreviewCropper> | null>(null)

/** PreviewCropper 正在导出时禁用按钮 */
const cropperBusy = computed<boolean>(() => cropperRef.value?.isExporting ?? false)

// ---------------------------------------------------------------------------
// 预览图 URL（直接使用 store 中的 previewImageUrl，无需 ArrayBuffer → Blob 转换）
// store 中 onCapturePreviewReady 回调已用 convertFileSrc 将磁盘路径转为 URL
// ---------------------------------------------------------------------------
const previewUrl = computed(() => captureStore.previewImageUrl)

// ---------------------------------------------------------------------------
// 初始化事件监听（store 幂等，重复调用安全）
// ---------------------------------------------------------------------------
onMounted(async () => {
  try {
    await captureStore.init()
  } catch (e) {
    console.error('[CaptureView] captureStore.init 失败:', e)
  }
})

// ---------------------------------------------------------------------------
// 按钮事件（开始/停止合并为单一按钮 F3）
// ---------------------------------------------------------------------------
async function onToggle() {
  if (captureStore.isRunning) {
    // 当前运行中 → 停止
    try {
      await captureStore.stop()
    } catch (e) {
      console.error('[CaptureView] stop 失败:', e)
    }
    return
  }
  // 当前未运行 → 开始
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
  // 仅自定义类别需要显式传入 rows/cols（其他类别由 Rust 端按区域配置解析）
  const isCustom = configStore.currentRegion?.category === '自定义'
  const rows = isCustom ? settingsStore.settings?.last_rows : undefined
  const cols = isCustom ? settingsStore.settings?.last_cols : undefined
  try {
    await captureStore.start(region, scrollMode, rows, cols)
  } catch (e) {
    console.error('[CaptureView] start 失败:', e)
  }
}

// ---------------------------------------------------------------------------
// PreviewCropper 事件处理
// ---------------------------------------------------------------------------
// 导出后不清空预览，直到下次开始截图才清理（在 captureStore.start 中处理）
/** 导出成功后仅记录日志，不清除预览 */
function onExported(payload: { path: string; cropped: boolean }) {
  console.log('[CaptureView] 导出成功:', payload.path)
}

/** 调用 PreviewCropper 暴露的导出裁剪方法 */
function onExportCropped() {
  cropperRef.value?.exportCropped()
}

/** 调用 PreviewCropper 暴露的导出原图方法 */
function onExportOriginal() {
  cropperRef.value?.exportOriginal()
}

/** 调用 PreviewCropper 暴露的清除选区方法 */
function onClearSelection() {
  cropperRef.value?.clearSelection()
}

// ---------------------------------------------------------------------------
// 左右分割线拖拽调节（逻辑由 useSplitter composable 提供）
//
// 设计：
//   - 默认左侧 200px，调整范围 150-400px
//   - 拖拽时实时更新 leftWidth，结束时持久化到 localStorage
//   - mounted/unmount/deactivate 时自动恢复与清理
// ---------------------------------------------------------------------------
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

/* 左下方使用说明 */
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

/* 右侧主体 */
.capture-right {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

/* 顶部工具栏：按钮组 + 截图信息 */
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

/* 中部预览区域 */
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