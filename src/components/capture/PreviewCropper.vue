<template>
  <div class="preview-cropper">
    <div
      ref="containerRef"
      class="cropper-canvas"
      tabindex="0"
      :class="{
        'cursor-crosshair': (mode === 'idle' || mode === 'drawing') && !spacePressed,
        'cursor-grab': mode === 'panning-ready' || (mode === 'idle' && spacePressed),
        'cursor-grabbing': mode === 'panning',
      }"
      @wheel.prevent="onWheel"
      @mousedown="onCanvasMouseDown"
      @auxclick="onAuxClick"
      @contextmenu.prevent
    >
      <div
        class="image-container"
        :style="{
          width: imgW + 'px',
          height: imgH + 'px',
          transform: `translate(${panX}px, ${panY}px)`,
        }"
      >
        <img
          :src="imageUrl"
          class="preview-img"
          @load="onImgLoad"
          draggable="false"
        />
        <!-- 绘制中且尺寸过小时不显示，避免点击瞬间 box-shadow 遮暗整图导致闪烁 -->
        <div
          v-if="selection && !(mode === 'drawing' && selection.w < 5 / zoom && selection.h < 5 / zoom)"
          class="selection-box"
          :style="selectionStyle"
          :class="{ 'cursor-move': mode === 'idle' }"
          @mousedown.stop="onSelectionMouseDown"
        >
          <div class="handle handle-nw" @mousedown.stop="onHandleDown($event, 'nw')"></div>
          <div class="handle handle-ne" @mousedown.stop="onHandleDown($event, 'ne')"></div>
          <div class="handle handle-sw" @mousedown.stop="onHandleDown($event, 'sw')"></div>
          <div class="handle handle-se" @mousedown.stop="onHandleDown($event, 'se')"></div>
          <div class="handle handle-n" @mousedown.stop="onHandleDown($event, 'n')"></div>
          <div class="handle handle-s" @mousedown.stop="onHandleDown($event, 's')"></div>
          <div class="handle handle-w" @mousedown.stop="onHandleDown($event, 'w')"></div>
          <div class="handle handle-e" @mousedown.stop="onHandleDown($event, 'e')"></div>
        </div>
      </div>

      <!-- 导出状态遮罩（画布中央） -->
      <div v-if="exporting || exportDone" class="export-overlay">
        <div v-if="exporting" class="spinner" />
        <p class="export-text">{{ exportDone ? t('capture.exportDone') : t('capture.exporting') }}</p>
        <p v-if="exportDone && exportPath" class="export-path" :title="exportPath">{{ exportPath }}</p>
      </div>
    </div>

    <div class="status-bar">
      <span class="status-item">{{ Math.round(zoom * 100) }}%</span>
      <span v-if="selection" class="status-item">{{ sizeTipText }}</span>
      <span v-if="captureStore.capturedRegion" class="status-item status-info">
        {{ captureStore.capturedRegion }} | {{ captureStore.capturedScrollMode }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
// PreviewCropper：截图预览区裁剪交互（左键=选区，中键=平移，滚轮=缩放，手柄=调整大小）
// pan/zoom 见 useImagePanZoom，选区拖拽见 useSelectionDrag，路径构建见 utils/exportPath
import { ref, computed } from 'vue'
import { api } from '@/api'
import { useCaptureStore } from '@/stores/capture.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useI18n } from '@/composables/useI18n'
import type { CropBox } from '@/types'
import { appDataDir, join } from '@tauri-apps/api/path'
import { formatTimestamp } from '@/utils/datetime'
import { useImagePanZoom } from '@/composables/useImagePanZoom'
import { useSelectionDrag } from '@/composables/useSelectionDrag'
import { buildSavePath as buildSavePathUtil } from '@/utils/exportPath'

interface Props {
  /** 图像 URL（来自 convertFileSrc，由父组件从磁盘路径转换） */
  imageUrl: string
  /** 当前会话 ID（用于回写 session 表） */
  sessionId?: number | null
}
const props = withDefaults(defineProps<Props>(), {
  sessionId: null,
})

const emit = defineEmits<{
  (e: 'exported', payload: { path: string; cropped: boolean }): void
}>()

const captureStore = useCaptureStore()
const settingsStore = useSettingsStore()
const { t } = useI18n()

// 画布容器 ref（供两个 composable 共用）
const containerRef = ref<HTMLDivElement | null>(null)

// 图像平移缩放（同时持有 mode / dragStart / spacePressed 等共享交互状态）
const panzoom = useImagePanZoom({ containerRef })
const {
  imgNaturalWidth,
  imgNaturalHeight,
  zoom,
  imgW,
  imgH,
  panX,
  panY,
  mode,
  spacePressed,
  onImgLoad,
  clientToImageCoords,
  isInsideImage,
  startPanning,
  handlePanning,
  onWheel,
} = panzoom

// 选区拖拽（drawing / moving / resizing + handles）
const {
  selection,
  onCanvasMouseDown,
  onAuxClick,
  onSelectionMouseDown,
  onHandleDown,
  clearSelection,
} = useSelectionDrag({
  containerRef,
  mode,
  dragStart: panzoom.dragStart,
  spacePressed,
  zoom,
  imgNaturalWidth,
  imgNaturalHeight,
  panX,
  panY,
  clientToImageCoords,
  isInsideImage,
  startPanning,
  handlePanning,
})

const exporting = ref(false)
const exportDone = ref(false)
/** 最近一次导出路径（用于在遮罩中显示） */
const exportPath = ref('')

const selectionStyle = computed(() => {
  const s = selection.value
  if (!s) return {}
  return {
    left: `${s.x * zoom.value}px`,
    top: `${s.y * zoom.value}px`,
    width: `${s.w * zoom.value}px`,
    height: `${s.h * zoom.value}px`,
  }
})

const sizeTipText = computed(() => {
  const s = selection.value
  if (!s || imgNaturalWidth.value === 0 || imgNaturalHeight.value === 0) return ''
  const pctW = ((s.w / imgNaturalWidth.value) * 100).toFixed(1)
  const pctH = ((s.h / imgNaturalHeight.value) * 100).toFixed(1)
  return `${Math.round(s.w)}×${Math.round(s.h)} px  ${pctW}%×${pctH}%`
})

function extractErrorMessage(e: unknown): string {
  if (e instanceof Error) return e.message
  if (typeof e === 'string') return e
  if (e && typeof e === 'object') {
    const obj = e as Record<string, unknown>
    if (typeof obj.message === 'string') return obj.message
    if (typeof obj.error === 'string') return obj.error
    try {
      return JSON.stringify(e)
    } catch {
      return String(e)
    }
  }
  return String(e)
}

function showError(prefix: string, e: unknown) {
  const msg = extractErrorMessage(e)
  console.error(`[PreviewCropper] ${prefix}:`, e)
  captureStore.setError(`${prefix}：${msg}`)
}

async function getSourcePath(): Promise<string | null> {
  try {
    const path = await api.getPreviewPath()
    if (path && path.length > 0) return path
    return null
  } catch (e) {
    console.error('[PreviewCropper] getPreviewPath 失败:', e)
    return null
  }
}

/**
 * 拼装保存裁剪路径：
 *   - 目录使用 screenshot_folder（空则回退到 appDataDir/screenshots）
 *   - 文件名使用 filename_pattern + '_crop' 后缀
 *   - 扩展名由 output_format 决定
 *   - 文件名冲突由 Rust 端 resolve_unique_path 自动添加 _1/_2 数字后缀
 */
async function resolveSavePath(): Promise<{ path: string; format: string }> {
  const fmt = settingsStore.settings?.output_format ?? 'JPG'
  const ts = formatTimestamp(new Date(), 'YYYYMMDD_HHMMSS')
  const region = captureStore.capturedRegion || ''
  const scrollMode = captureStore.capturedScrollMode || ''

  const pattern = settingsStore.settings?.filename_pattern || '{region}_{timestamp}_{scrollMode}'
  const vars = {
    prefix: 'stitched',
    timestamp: ts,
    region,
    scrollMode,
  }

  let folder = settingsStore.settings?.screenshot_folder ?? ''
  if (!folder) {
    const dataDir = await appDataDir()
    folder = await join(dataDir, 'screenshots')
  }
  const path = buildSavePathUtil(folder, pattern, vars, fmt, '_crop')
  return { path, format: fmt }
}

/** 显示导出完成提示（3秒后自动消失，并清空路径） */
function showExportDone(path: string) {
  exportPath.value = path
  exportDone.value = true
  setTimeout(() => {
    exportDone.value = false
    exportPath.value = ''
  }, 3000)
}

async function onSaveCropped() {
  if (exporting.value) return
  const s = selection.value
  if (!s) return
  exporting.value = true
  try {
    const sourcePath = await getSourcePath()
    if (!sourcePath) {
      showError('保存失败', new Error('无法获取源图像路径，请重新截图'))
      return
    }
    const crop: CropBox = {
      x: Math.round(s.x),
      y: Math.round(s.y),
      w: Math.round(s.w),
      h: Math.round(s.h),
    }
    if (crop.w <= 0 || crop.h <= 0) {
      showError('裁剪失败', new Error('选区尺寸无效'))
      return
    }
    const { path, format } = await resolveSavePath()
    const quality = settingsStore.settings?.jpg_quality ?? 95
    const cropBoxJson = JSON.stringify(crop)
    const outPath = await api.exportImage(
      sourcePath,
      crop,
      format,
      quality,
      path,
      props.sessionId,
      cropBoxJson,
    )
    emit('exported', { path: outPath, cropped: true })
    showExportDone(outPath)
  } catch (e) {
    showError('保存裁剪失败', e)
  } finally {
    exporting.value = false
  }
}

defineExpose({
  saveCropped: onSaveCropped,
  clearSelection,
  isExporting: exporting,
})
</script>

<style scoped>
.preview-cropper {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  background: var(--bg-primary);
  overflow: hidden;
}

/* 画布区域 */
.cropper-canvas {
  flex: 1;
  position: relative;
  overflow: hidden;
  background: #1a1a1a;
  background-image:
    linear-gradient(45deg, #2a2a2a 25%, transparent 25%),
    linear-gradient(-45deg, #2a2a2a 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, #2a2a2a 75%),
    linear-gradient(-45deg, transparent 75%, #2a2a2a 75%);
  background-size: 16px 16px;
  background-position: 0 0, 0 8px, 8px -8px, -8px 0;
  user-select: none;
  cursor: grab;
}

.cropper-canvas.cursor-crosshair {
  cursor: crosshair;
}

.cropper-canvas.cursor-grab {
  cursor: grab;
}

.cropper-canvas.cursor-grabbing {
  cursor: grabbing;
}

.image-container {
  position: absolute;
  top: 0;
  left: 0;
  transform-origin: 0 0;
  overflow: visible;
  will-change: transform, width, height;
}

.preview-img {
  display: block;
  width: 100%;
  height: 100%;
  user-select: none;
  -webkit-user-drag: none;
  pointer-events: none;
}

/* 选区框 */
.selection-box {
  position: absolute;
  border: 1px solid var(--accent);
  background: transparent;
  box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.5);
  pointer-events: auto;
  z-index: 10;
}

/* 四角手柄 */
.handle {
  position: absolute;
  width: 8px;
  height: 8px;
  background: #ffffff;
  border: 1px solid var(--accent);
  border-radius: 2px;
  z-index: 11;
}

.handle-nw { top: -4px; left: -4px; cursor: nwse-resize; }
.handle-ne { top: -4px; right: -4px; cursor: nesw-resize; }
.handle-sw { bottom: -4px; left: -4px; cursor: nesw-resize; }
.handle-se { bottom: -4px; right: -4px; cursor: nwse-resize; }

/* 四边手柄 */
.handle-n { top: -3px; left: 50%; transform: translateX(-50%); width: 16px; height: 4px; cursor: ns-resize; }
.handle-s { bottom: -3px; left: 50%; transform: translateX(-50%); width: 16px; height: 4px; cursor: ns-resize; }
.handle-w { left: -3px; top: 50%; transform: translateY(-50%); width: 4px; height: 16px; cursor: ew-resize; }
.handle-e { right: -3px; top: 50%; transform: translateY(-50%); width: 4px; height: 16px; cursor: ew-resize; }

.cursor-move { cursor: move; }

/* 导出遮罩 */
.export-overlay {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  background: rgba(0, 0, 0, 0.85);
  padding: 28px 48px;
  border-radius: 8px;
  z-index: 100;
  pointer-events: none;
}

.export-overlay .spinner {
  width: 32px;
  height: 32px;
  border: 3px solid rgba(255, 255, 255, 0.2);
  border-top-color: #ffffff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.export-text {
  color: #ffffff;
  font-size: 14px;
  font-weight: 500;
  margin: 0;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  white-space: nowrap;
}

.export-path {
  color: rgba(255, 255, 255, 0.75);
  font-size: 11px;
  margin: 0;
  max-width: 360px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: "Consolas", "Monaco", monospace;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
}

/* 底部状态栏 */
.status-bar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 4px 12px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border);
  flex-shrink: 0;
  font-size: 11px;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
}

.status-item {
  white-space: nowrap;
}

.status-item.status-info {
  margin-left: auto;
  color: var(--text-secondary);
}
</style>
