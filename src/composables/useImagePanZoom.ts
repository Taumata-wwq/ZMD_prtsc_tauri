/**
 * 图像平移与缩放 composable
 * 提供图像加载、自适应缩放、平移、ResizeObserver、鼠标/滚轮缩放。
 * mode / dragStart / spacePressed 供 useSelectionDrag 读写共享交互状态。
 */
import { ref, computed, onMounted, onBeforeUnmount, onActivated, nextTick } from 'vue'
import type { Ref, ComputedRef } from 'vue'
import { clamp } from '@/utils/math'

export type InteractionMode = 'idle' | 'drawing' | 'moving' | 'resizing' | 'panning-ready' | 'panning'

/** mousedown 时记录的交互起始状态（普通对象，外部可直接修改属性） */
export interface DragStartState {
  mouseImgX: number
  mouseImgY: number
  selX: number
  selY: number
  selW: number
  selH: number
  panX: number
  panY: number
  mouseX: number
  mouseY: number
}

interface UseImagePanZoomOptions {
  containerRef: Ref<HTMLDivElement | null>
}

export function useImagePanZoom(options: UseImagePanZoomOptions) {
  const { containerRef } = options

  const imgNaturalWidth = ref(0)
  const imgNaturalHeight = ref(0)
  const imgLoaded = ref(false)
  const zoom = ref(1)
  const imgW: ComputedRef<number> = computed(() => imgNaturalWidth.value * zoom.value)
  const imgH: ComputedRef<number> = computed(() => imgNaturalHeight.value * zoom.value)
  const panX = ref(0)
  const panY = ref(0)
  let fitZoom = 1 // 自适应缩放（图像完全可见时的 zoom），非响应式

  const mode = ref<InteractionMode>('idle')
  const dragStart: DragStartState = { mouseImgX: 0, mouseImgY: 0, selX: 0, selY: 0, selW: 0, selH: 0, panX: 0, panY: 0, mouseX: 0, mouseY: 0 }
  const spacePressed = ref(false)

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
    return Math.min(cw / imgNaturalWidth.value, ch / imgNaturalHeight.value, 1)
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
    // keep-alive 切换时容器尺寸可能为 0，跳过避免 panX/panY 计算为负值
    if (cw <= 0 || ch <= 0) return
    panX.value = (cw - imgW.value) / 2
    panY.value = (ch - imgH.value) / 2
  }

  function clientToImageCoords(clientX: number, clientY: number): { x: number; y: number } {
    const container = containerRef.value
    if (!container) return { x: 0, y: 0 }
    const rect = container.getBoundingClientRect()
    return {
      x: (clientX - rect.left - panX.value) / zoom.value,
      y: (clientY - rect.top - panY.value) / zoom.value,
    }
  }

  function isInsideImage(x: number, y: number): boolean {
    return x >= 0 && y >= 0 && x <= imgNaturalWidth.value && y <= imgNaturalHeight.value
  }

  // 平移（中键 / 空格+左键 触发；mousemove/mouseup 由 useSelectionDrag 注册）
  function startPanning(clientX: number, clientY: number) {
    mode.value = 'panning'
    dragStart.panX = panX.value
    dragStart.panY = panY.value
    dragStart.mouseX = clientX
    dragStart.mouseY = clientY
  }

  function handlePanning(e: MouseEvent) {
    panX.value = dragStart.panX + (e.clientX - dragStart.mouseX)
    panY.value = dragStart.panY + (e.clientY - dragStart.mouseY)
  }

  // 滚轮缩放（rAF 节流）
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

    const factor = -e.deltaY > 0 ? 1.1 : 0.9
    const minZoom = Math.max(0.05, fitZoom * 0.5)
    const newZoom = clamp(zoom.value * factor, minZoom, 5)
    if (newZoom === zoom.value) return

    zoom.value = newZoom
    panX.value = mxContainer - mxImg * newZoom
    panY.value = myContainer - myImg * newZoom
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

  // 空格键监听（空格+左键拖拽画布）
  function isEditableTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false
    const tag = target.tagName.toLowerCase()
    return tag === 'input' || tag === 'textarea' || tag === 'select' || target.isContentEditable
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.code === 'Space' && !isEditableTarget(e.target)) {
      spacePressed.value = true
      e.preventDefault()
    }
  }

  function onKeyUp(e: KeyboardEvent) {
    if (e.code === 'Space') spacePressed.value = false
  }

  // ResizeObserver + keep-alive 重新激活
  let resizeObserver: ResizeObserver | null = null

  onMounted(() => {
    if (containerRef.value) {
      resizeObserver = new ResizeObserver(() => {
        if (!imgLoaded.value || mode.value !== 'idle') return
        const container = containerRef.value
        if (!container) return
        const cw = container.clientWidth
        const ch = container.clientHeight
        if (cw <= 0 || ch <= 0) return
        const oldFitZoom = fitZoom
        const newFitZoom = computeFitZoom()
        if (newFitZoom <= 0) return
        // 当前 zoom 等于旧 fitZoom 时跟随更新（保持图像铺满可视区）
        if (Math.abs(zoom.value - oldFitZoom) < 0.001) {
          fitZoom = newFitZoom
          zoom.value = newFitZoom
          centerImage()
          return
        }
        fitZoom = newFitZoom
        // 图像完全离开视口时重新居中
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

  // keep-alive 重新激活时容器尺寸可能从 0 恢复，等 DOM 更新后重算 fitZoom
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
    if (resizeObserver) {
      resizeObserver.disconnect()
      resizeObserver = null
    }
    window.removeEventListener('keydown', onKeyDown)
    window.removeEventListener('keyup', onKeyUp)
  })

  return {
    imgNaturalWidth,
    imgNaturalHeight,
    imgLoaded,
    zoom,
    imgW,
    imgH,
    panX,
    panY,
    mode,
    dragStart,
    spacePressed,
    onImgLoad,
    resetView,
    centerImage,
    clientToImageCoords,
    isInsideImage,
    startPanning,
    handlePanning,
    onWheel,
  }
}
