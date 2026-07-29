<template>
  <div class="preview-cropper">
    <!-- 画布区域 -->
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
        <!-- 选区框 -->
        <!-- 绘制中且尺寸过小时不显示，避免点击瞬间 box-shadow 遮暗整图导致闪烁 -->
        <div
          v-if="selection && !(mode === 'drawing' && selection.w < 5 / zoom && selection.h < 5 / zoom)"
          class="selection-box"
          :style="selectionStyle"
          :class="{ 'cursor-move': mode === 'idle' }"
          @mousedown.stop="onSelectionMouseDown"
        >
          <!-- 四角手柄 -->
          <div class="handle handle-nw" @mousedown.stop="onHandleDown($event, 'nw')"></div>
          <div class="handle handle-ne" @mousedown.stop="onHandleDown($event, 'ne')"></div>
          <div class="handle handle-sw" @mousedown.stop="onHandleDown($event, 'sw')"></div>
          <div class="handle handle-se" @mousedown.stop="onHandleDown($event, 'se')"></div>
          <!-- 四边手柄 -->
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
      </div>
    </div>

    <!-- 底部状态栏：缩放比例 + 选区尺寸 + 截图信息 -->
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
// PreviewCropper：截图预览区裁剪交互
// 左键拖拽=选区，中键拖拽=平移，滚轮=缩放，四角/四边手柄=调整大小
import { ref, computed, onMounted, onBeforeUnmount, onActivated, nextTick } from 'vue'
import { api } from '@/api'
import { useCaptureStore } from '@/stores/capture.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useI18n } from '@/composables/useI18n'
import type { CropBox } from '@/types'
import { appDataDir, join } from '@tauri-apps/api/path'

interface Props {
  /** 图像 URL（来自 convertFileSrc，由父组件从磁盘路径转换） */
  imageUrl: string
  /** 截图来源：'auto' 自动拼接 | 'manual' 手动单张 */
  source?: 'auto' | 'manual'
  /** 当前会话 ID（用于回写 session 表） */
  sessionId?: number | null
}
const props = withDefaults(defineProps<Props>(), {
  source: 'auto',
  sessionId: null,
})

const emit = defineEmits<{
  (e: 'exported', payload: { path: string; cropped: boolean }): void
}>()

const captureStore = useCaptureStore()
const settingsStore = useSettingsStore()
const { t } = useI18n()

// ---------------------------------------------------------------------------
// DOM refs
// ---------------------------------------------------------------------------
const containerRef = ref<HTMLDivElement | null>(null)

// ---------------------------------------------------------------------------
// 图像状态
// ---------------------------------------------------------------------------
const imgNaturalWidth = ref(0)
const imgNaturalHeight = ref(0)
const imgLoaded = ref(false)

/** 缩放因子（1 = 原图尺寸，0.5 = 缩小一半） */
const zoom = ref(1)
/** 图像显示尺寸（naturalWidth * zoom） */
const imgW = computed(() => imgNaturalWidth.value * zoom.value)
const imgH = computed(() => imgNaturalHeight.value * zoom.value)

/** 图像平移（容器坐标系） */
const panX = ref(0)
const panY = ref(0)

/** 自适应缩放（图像完全可见） */
let fitZoom = 1

// ---------------------------------------------------------------------------
// 选区状态（原图像素坐标）
// ---------------------------------------------------------------------------
interface Selection {
  x: number
  y: number
  w: number
  h: number
}
const selection = ref<Selection | null>(null)

/** 交互模式 */
type Mode = 'idle' | 'drawing' | 'moving' | 'resizing' | 'panning-ready' | 'panning'
const mode = ref<Mode>('idle')
/** resize 时记录哪个角/边 */
type Handle = 'nw' | 'ne' | 'sw' | 'se' | 'n' | 's' | 'w' | 'e'
let resizeHandle: Handle = 'se'
/** 交互起始状态（mousedown 时记录） */
let dragStart = {
  mouseImgX: 0,
  mouseImgY: 0,
  selX: 0,
  selY: 0,
  selW: 0,
  selH: 0,
  panX: 0,
  panY: 0,
  mouseX: 0,
  mouseY: 0,
}

/** 空格键按下状态（空格+左键拖拽画布） */
const spacePressed = ref(false)

// ---------------------------------------------------------------------------
// 导出状态
// ---------------------------------------------------------------------------
const exporting = ref(false)
const exportDone = ref(false)

// ---------------------------------------------------------------------------
// 选区渲染样式
// ---------------------------------------------------------------------------
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

// =========================================================================
// 图像加载与自适应
// =========================================================================
function onImgLoad(e: Event) {
  const img = e.target as HTMLImageElement
  imgNaturalWidth.value = img.naturalWidth
  imgNaturalHeight.value = img.naturalHeight
  imgLoaded.value = true
  resetView()
}

function computeFitZoom(): number {
  const container = containerRef.value
  if (!container || imgNaturalWidth.value === 0) return 1
  const cw = container.clientWidth - 24
  const ch = container.clientHeight - 24
  if (cw <= 0 || ch <= 0) return 1
  const sx = cw / imgNaturalWidth.value
  const sy = ch / imgNaturalHeight.value
  return Math.min(sx, sy, 1)
}

function resetView() {
  fitZoom = computeFitZoom()
  zoom.value = fitZoom
  centerImage()
}

function centerImage() {
  const container = containerRef.value
  if (!container) return
  const cw = container.clientWidth
  const ch = container.clientHeight
  // 防御 keep-alive 切换时容器尺寸为 0，导致 panX/panY 计算为负值
  if (cw <= 0 || ch <= 0) return
  panX.value = (cw - imgW.value) / 2
  panY.value = (ch - imgH.value) / 2
}

// =========================================================================
// 坐标转换
// =========================================================================
function clientToImageCoords(clientX: number, clientY: number): { x: number; y: number } {
  const container = containerRef.value
  if (!container) return { x: 0, y: 0 }
  const rect = container.getBoundingClientRect()
  const cx = clientX - rect.left - panX.value
  const cy = clientY - rect.top - panY.value
  return { x: cx / zoom.value, y: cy / zoom.value }
}

// =========================================================================
// 鼠标交互
// =========================================================================

function onCanvasMouseDown(e: MouseEvent) {
  containerRef.value?.focus()
  // 中键 → 平移模式
  if (e.button === 1) {
    startPanning(e.clientX, e.clientY)
    e.preventDefault()
    return
  }
  if (e.button !== 0) return

  // 空格+左键 → 平移画布
  if (spacePressed.value) {
    startPanning(e.clientX, e.clientY)
    e.preventDefault()
    return
  }

  const img = clientToImageCoords(e.clientX, e.clientY)
  if (!isInsideImage(img.x, img.y)) return

  mode.value = 'drawing'
  selection.value = { x: img.x, y: img.y, w: 0, h: 0 }
  dragStart = {
    mouseImgX: img.x,
    mouseImgY: img.y,
    selX: img.x,
    selY: img.y,
    selW: 0,
    selH: 0,
    panX: panX.value,
    panY: panY.value,
    mouseX: e.clientX,
    mouseY: e.clientY,
  }
  e.preventDefault()
  startDragListeners()
}

function onAuxClick(e: MouseEvent) {
  if (e.button === 1) {
    e.preventDefault()
  }
}

function startPanning(clientX: number, clientY: number) {
  mode.value = 'panning'
  dragStart = {
    ...dragStart,
    panX: panX.value,
    panY: panY.value,
    mouseX: clientX,
    mouseY: clientY,
  }
  startDragListeners()
}

function onSelectionMouseDown(e: MouseEvent) {
  if (e.button === 1) {
    startPanning(e.clientX, e.clientY)
    e.preventDefault()
    return
  }
  if (e.button !== 0) return
  if (!selection.value) return

  if (spacePressed.value) {
    startPanning(e.clientX, e.clientY)
    e.preventDefault()
    return
  }

  mode.value = 'moving'
  const img = clientToImageCoords(e.clientX, e.clientY)
  dragStart = {
    ...dragStart,
    mouseImgX: img.x,
    mouseImgY: img.y,
    selX: selection.value.x,
    selY: selection.value.y,
    selW: selection.value.w,
    selH: selection.value.h,
  }
  e.preventDefault()
  startDragListeners()
}

function onHandleDown(e: MouseEvent, handle: Handle) {
  if (e.button !== 0) return
  if (!selection.value) return

  if (spacePressed.value) {
    startPanning(e.clientX, e.clientY)
    e.preventDefault()
    e.stopPropagation()
    return
  }

  mode.value = 'resizing'
  resizeHandle = handle
  const img = clientToImageCoords(e.clientX, e.clientY)
  dragStart = {
    ...dragStart,
    mouseImgX: img.x,
    mouseImgY: img.y,
    selX: selection.value.x,
    selY: selection.value.y,
    selW: selection.value.w,
    selH: selection.value.h,
  }
  e.preventDefault()
  e.stopPropagation()
  startDragListeners()
}

function onMouseMove(e: MouseEvent) {
  if (mode.value === 'idle') return

  if (mode.value === 'panning') {
    const dx = e.clientX - dragStart.mouseX
    const dy = e.clientY - dragStart.mouseY
    panX.value = dragStart.panX + dx
    panY.value = dragStart.panY + dy
    return
  }

  const img = clientToImageCoords(e.clientX, e.clientY)
  const curX = clamp(img.x, 0, imgNaturalWidth.value)
  const curY = clamp(img.y, 0, imgNaturalHeight.value)

  if (mode.value === 'drawing') {
    const x = Math.min(dragStart.mouseImgX, curX)
    const y = Math.min(dragStart.mouseImgY, curY)
    const w = Math.abs(curX - dragStart.mouseImgX)
    const h = Math.abs(curY - dragStart.mouseImgY)
    selection.value = { x, y, w, h }
  } else if (mode.value === 'moving' && selection.value) {
    const dx = img.x - dragStart.mouseImgX
    const dy = img.y - dragStart.mouseImgY
    let newX = dragStart.selX + dx
    let newY = dragStart.selY + dy
    newX = clamp(newX, 0, imgNaturalWidth.value - dragStart.selW)
    newY = clamp(newY, 0, imgNaturalHeight.value - dragStart.selH)
    selection.value = {
      x: newX,
      y: newY,
      w: dragStart.selW,
      h: dragStart.selH,
    }
  } else if (mode.value === 'resizing' && selection.value) {
    const dx = img.x - dragStart.mouseImgX
    const dy = img.y - dragStart.mouseImgY
    let { selX, selY, selW, selH } = dragStart
    let newX = selX
    let newY = selY
    let newW = selW
    let newH = selH
    switch (resizeHandle) {
      case 'nw':
        newX = selX + dx
        newY = selY + dy
        newW = selW - dx
        newH = selH - dy
        break
      case 'ne':
        newY = selY + dy
        newW = selW + dx
        newH = selH - dy
        break
      case 'sw':
        newX = selX + dx
        newW = selW - dx
        newH = selH + dy
        break
      case 'se':
        newW = selW + dx
        newH = selH + dy
        break
      case 'n':
        newY = selY + dy
        newH = selH - dy
        break
      case 's':
        newH = selH + dy
        break
      case 'w':
        newX = selX + dx
        newW = selW - dx
        break
      case 'e':
        newW = selW + dx
        break
    }
    if (newW < 0) {
      newX = newX + newW
      newW = -newW
    }
    if (newH < 0) {
      newY = newY + newH
      newH = -newH
    }
    newX = Math.max(0, newX)
    newY = Math.max(0, newY)
    if (newX + newW > imgNaturalWidth.value) newW = imgNaturalWidth.value - newX
    if (newY + newH > imgNaturalHeight.value) newH = imgNaturalHeight.value - newY
    selection.value = { x: newX, y: newY, w: newW, h: newH }
  }
}

function onMouseUp() {
  if (mode.value === 'drawing' && selection.value) {
    if (selection.value.w < 5 / zoom.value || selection.value.h < 5 / zoom.value) {
      selection.value = null
    }
  }
  mode.value = 'idle'
  stopDragListeners()
}

// ---------------------------------------------------------------------------
// 滚轮缩放
// ---------------------------------------------------------------------------
let pendingWheelEvent: WheelEvent | null = null
let wheelRafId: number | null = null

function applyWheel(e: WheelEvent) {
  const container = containerRef.value
  if (!container) return
  const rect = container.getBoundingClientRect()
  const mxContainer = e.clientX - rect.left
  const myContainer = e.clientY - rect.top
  const mxImg = (mxContainer - panX.value) / zoom.value
  const myImg = (myContainer - panY.value) / zoom.value

  const delta = -e.deltaY
  const step = 0.1
  const factor = delta > 0 ? 1 + step : 1 - step
  let newZoom = zoom.value * factor
  const minZoom = Math.max(0.05, fitZoom * 0.5)
  const maxZoom = 5
  newZoom = clamp(newZoom, minZoom, maxZoom)
  if (newZoom === zoom.value) return

  const newPanX = mxContainer - mxImg * newZoom
  const newPanY = myContainer - myImg * newZoom

  zoom.value = newZoom
  panX.value = newPanX
  panY.value = newPanY
}

function onWheel(e: WheelEvent) {
  if (!imgLoaded.value) return
  e.preventDefault()
  pendingWheelEvent = e
  if (wheelRafId === null) {
    wheelRafId = requestAnimationFrame(() => {
      wheelRafId = null
      if (pendingWheelEvent) {
        applyWheel(pendingWheelEvent)
        pendingWheelEvent = null
      }
    })
  }
}

// =========================================================================
// 辅助函数
// =========================================================================
function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v))
}

function isInsideImage(x: number, y: number): boolean {
  return x >= 0 && y >= 0 && x <= imgNaturalWidth.value && y <= imgNaturalHeight.value
}

function clearSelection() {
  selection.value = null
  mode.value = 'idle'
}

// =========================================================================
// 错误处理
// =========================================================================
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

// =========================================================================
// 导出（使用 source_path，使用 filename_pattern 自定义文件名）
// =========================================================================
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

/** 生成时间戳：YYYYMMDD_HHMMSS */
function timestamp(): string {
  const d = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return (
    `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}` +
    `_${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}`
  )
}

/**
 * 拼装输出路径：使用 filename_pattern 设置项自定义文件名
 * 可用占位符：{timestamp} {region} {scrollMode}（{prefix} 仍兼容旧设置）
 */
async function buildOutputPath(): Promise<{ path: string; format: string; ext: string }> {
  const fmt = settingsStore.settings?.output_format ?? 'JPG'
  const ext = fmt.toLowerCase()
  const ts = timestamp()
  const prefix = props.source === 'manual' ? 'manual' : 'stitched'
  // 使用截图时锁定的区域和滚动次数（防止左侧变动导致命名错误）
  const region = captureStore.capturedRegion || ''
  const scrollMode = captureStore.capturedScrollMode || ''

  const pattern = settingsStore.settings?.filename_pattern || '{region}_{timestamp}_{scrollMode}'
  const filename = pattern
    .replace(/\{prefix\}/g, prefix)
    .replace(/\{timestamp\}/g, ts)
    .replace(/\{region\}/g, region)
    .replace(/\{scrollMode\}/g, scrollMode)
    + '.' + ext

  let folder = settingsStore.settings?.output_folder ?? ''
  if (!folder) {
    const dataDir = await appDataDir()
    folder = await join(dataDir, 'screenshots')
  }
  const sep = folder.endsWith('/') || folder.endsWith('\\') ? '' : '/'
  return { path: `${folder}${sep}${filename}`, format: fmt, ext }
}

/** 显示导出完成提示（2秒后自动消失） */
function showExportDone() {
  exportDone.value = true
  setTimeout(() => {
    exportDone.value = false
  }, 2000)
}

async function onExportOriginal() {
  if (exporting.value) return
  exporting.value = true
  try {
    const sourcePath = await getSourcePath()
    if (!sourcePath) {
      showError('导出失败', new Error('无法获取源图像路径，请重新截图'))
      return
    }
    const { path, format } = await buildOutputPath()
    const quality = settingsStore.settings?.jpg_quality ?? 95
    const outPath = await api.exportImage(
      sourcePath,
      null,
      format,
      quality,
      path,
      props.sessionId,
      null,
    )
    emit('exported', { path: outPath, cropped: false })
    showExportDone()
  } catch (e) {
    showError('导出原图失败', e)
  } finally {
    exporting.value = false
  }
}

async function onExportCropped() {
  if (exporting.value) return
  const s = selection.value
  if (!s) return
  exporting.value = true
  try {
    const sourcePath = await getSourcePath()
    if (!sourcePath) {
      showError('导出失败', new Error('无法获取源图像路径，请重新截图'))
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
    const { path, format } = await buildOutputPath()
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
    showExportDone()
  } catch (e) {
    showError('裁剪后导出失败', e)
  } finally {
    exporting.value = false
  }
}

// =========================================================================
// 全局事件监听
// =========================================================================
function startDragListeners() {
  window.addEventListener('mousemove', onMouseMove)
  window.addEventListener('mouseup', onMouseUp)
}

function stopDragListeners() {
  window.removeEventListener('mousemove', onMouseMove)
  window.removeEventListener('mouseup', onMouseUp)
}

function onKeyDown(e: KeyboardEvent) {
  if (e.code === 'Space' && !isEditableTarget(e.target)) {
    spacePressed.value = true
    e.preventDefault()
  }
}

function onKeyUp(e: KeyboardEvent) {
  if (e.code === 'Space') {
    spacePressed.value = false
  }
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  const tag = target.tagName.toLowerCase()
  return tag === 'input' || tag === 'textarea' || tag === 'select' || target.isContentEditable
}

let resizeObserver: ResizeObserver | null = null

onMounted(() => {
  if (containerRef.value) {
    resizeObserver = new ResizeObserver(() => {
      if (!imgLoaded.value || mode.value !== 'idle') return
      const container = containerRef.value
      if (!container) return
      const cw = container.clientWidth
      const ch = container.clientHeight
      // 防御 keep-alive 切换时容器尺寸为 0（隐藏状态），跳过避免错误重置
      if (cw <= 0 || ch <= 0) return
      const oldFitZoom = fitZoom
      const newFitZoom = computeFitZoom()
      if (newFitZoom <= 0) return
      if (Math.abs(zoom.value - oldFitZoom) < 0.001) {
        fitZoom = newFitZoom
        zoom.value = newFitZoom
        centerImage()
        return
      }
      fitZoom = newFitZoom
      const imgLeft = panX.value
      const imgTop = panY.value
      const imgRight = panX.value + imgW.value
      const imgBottom = panY.value + imgH.value
      if (imgRight < 0 || imgLeft > cw || imgBottom < 0 || imgTop > ch) {
        centerImage()
      }
    })
    resizeObserver.observe(containerRef.value)
  }
  window.addEventListener('keydown', onKeyDown)
  window.addEventListener('keyup', onKeyUp)
})

// keep-alive 重新激活时，容器尺寸可能从 0 恢复为正常值
// 等待 DOM 更新后重新计算 fitZoom 并保持图像居中
onActivated(() => {
  if (!imgLoaded.value) return
  void nextTick(() => {
    const container = containerRef.value
    if (!container || container.clientWidth <= 0 || container.clientHeight <= 0) return
    const newFitZoom = computeFitZoom()
    if (newFitZoom <= 0) return
    fitZoom = newFitZoom
    zoom.value = newFitZoom
    centerImage()
  })
})

onBeforeUnmount(() => {
  stopDragListeners()
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
  window.removeEventListener('keydown', onKeyDown)
  window.removeEventListener('keyup', onKeyUp)
})

// 暴露方法供父组件调用
defineExpose({
  exportCropped: onExportCropped,
  exportOriginal: onExportOriginal,
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
  background: rgba(0, 0, 0, 0.75);
  padding: 24px 40px;
  border-radius: 8px;
  z-index: 20;
}

.export-overlay .spinner {
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

.export-text {
  color: var(--text-primary);
  font-size: 14px;
  margin: 0;
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