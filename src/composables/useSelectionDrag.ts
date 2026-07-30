/**
 * 选区拖拽 composable
 * 负责 selection 的 drawing / moving / resizing 三种交互模式的状态机，
 * 以及全局 mousemove / mouseup 监听的注册与清理。
 * 中键 / 空格+左键触发的平移委托给 useImagePanZoom。
 */
import { ref, onBeforeUnmount } from 'vue'
import type { Ref } from 'vue'
import { clamp } from '@/utils/math'
import type { InteractionMode, DragStartState } from './useImagePanZoom'

/** 选区（原图像素坐标） */
export interface Selection {
  x: number
  y: number
  w: number
  h: number
}

export type Handle = 'nw' | 'ne' | 'sw' | 'se' | 'n' | 's' | 'w' | 'e'

interface UseSelectionDragOptions {
  containerRef: Ref<HTMLDivElement | null>
  mode: Ref<InteractionMode>
  dragStart: DragStartState
  spacePressed: Ref<boolean>
  zoom: Ref<number>
  imgNaturalWidth: Ref<number>
  imgNaturalHeight: Ref<number>
  panX: Ref<number>
  panY: Ref<number>
  clientToImageCoords: (clientX: number, clientY: number) => { x: number; y: number }
  isInsideImage: (x: number, y: number) => boolean
  startPanning: (clientX: number, clientY: number) => void
  handlePanning: (e: MouseEvent) => void
}

export function useSelectionDrag(options: UseSelectionDragOptions) {
  const {
    containerRef, mode, dragStart, spacePressed, zoom,
    imgNaturalWidth, imgNaturalHeight, panX, panY,
    clientToImageCoords, isInsideImage, startPanning, handlePanning,
  } = options

  const selection = ref<Selection | null>(null)
  let resizeHandle: Handle = 'se'

  /** 平移入口：进入 panning 模式并注册全局监听 */
  function beginPanning(clientX: number, clientY: number) {
    startPanning(clientX, clientY)
    startDragListeners()
  }

  /** 记录 dragStart 起始状态（drawing 与 moving/resizing 共用） */
  function recordDragStart(e: MouseEvent, sel: { x: number; y: number; w: number; h: number }) {
    const img = clientToImageCoords(e.clientX, e.clientY)
    Object.assign(dragStart, {
      mouseImgX: img.x,
      mouseImgY: img.y,
      selX: sel.x,
      selY: sel.y,
      selW: sel.w,
      selH: sel.h,
      panX: panX.value,
      panY: panY.value,
      mouseX: e.clientX,
      mouseY: e.clientY,
    })
  }

  /** 画布 mousedown：中键/空格+左键=平移，否则=开始绘制选区 */
  function onCanvasMouseDown(e: MouseEvent) {
    containerRef.value?.focus()
    if (e.button === 1 || (e.button === 0 && spacePressed.value)) {
      beginPanning(e.clientX, e.clientY)
      e.preventDefault()
      return
    }
    if (e.button !== 0) return

    const img = clientToImageCoords(e.clientX, e.clientY)
    if (!isInsideImage(img.x, img.y)) return

    mode.value = 'drawing'
    selection.value = { x: img.x, y: img.y, w: 0, h: 0 }
    recordDragStart(e, { x: img.x, y: img.y, w: 0, h: 0 })
    e.preventDefault()
    startDragListeners()
  }

  /** 画布 auxclick：屏蔽中键默认行为 */
  function onAuxClick(e: MouseEvent) {
    if (e.button === 1) {
      e.preventDefault()
    }
  }

  /** 选区 mousedown：中键/空格+左键=平移，否则=开始移动选区 */
  function onSelectionMouseDown(e: MouseEvent) {
    if (e.button === 1 || (e.button === 0 && spacePressed.value)) {
      beginPanning(e.clientX, e.clientY)
      e.preventDefault()
      return
    }
    if (e.button !== 0 || !selection.value) return

    mode.value = 'moving'
    recordDragStart(e, selection.value)
    e.preventDefault()
    startDragListeners()
  }

  /** 手柄 mousedown：开始 resize（空格+左键则改走平移） */
  function onHandleDown(e: MouseEvent, handle: Handle) {
    if (e.button !== 0 || !selection.value) return

    if (spacePressed.value) {
      beginPanning(e.clientX, e.clientY)
      e.preventDefault()
      e.stopPropagation()
      return
    }

    mode.value = 'resizing'
    resizeHandle = handle
    recordDragStart(e, selection.value)
    e.preventDefault()
    e.stopPropagation()
    startDragListeners()
  }

  function onMouseMove(e: MouseEvent) {
    if (mode.value === 'idle') return
    switch (mode.value) {
      case 'panning': handlePanning(e); break
      case 'drawing': handleDrawing(e); break
      case 'moving': handleMoving(e); break
      case 'resizing': handleResizing(e); break
    }
  }

  function handleDrawing(e: MouseEvent) {
    const img = clientToImageCoords(e.clientX, e.clientY)
    const curX = clamp(img.x, 0, imgNaturalWidth.value)
    const curY = clamp(img.y, 0, imgNaturalHeight.value)
    selection.value = {
      x: Math.min(dragStart.mouseImgX, curX),
      y: Math.min(dragStart.mouseImgY, curY),
      w: Math.abs(curX - dragStart.mouseImgX),
      h: Math.abs(curY - dragStart.mouseImgY),
    }
  }

  function handleMoving(e: MouseEvent) {
    if (!selection.value) return
    const img = clientToImageCoords(e.clientX, e.clientY)
    const dx = img.x - dragStart.mouseImgX
    const dy = img.y - dragStart.mouseImgY
    selection.value = {
      x: clamp(dragStart.selX + dx, 0, imgNaturalWidth.value - dragStart.selW),
      y: clamp(dragStart.selY + dy, 0, imgNaturalHeight.value - dragStart.selH),
      w: dragStart.selW,
      h: dragStart.selH,
    }
  }

  function handleResizing(e: MouseEvent) {
    if (!selection.value) return
    const img = clientToImageCoords(e.clientX, e.clientY)
    const dx = img.x - dragStart.mouseImgX
    const dy = img.y - dragStart.mouseImgY
    const { selX, selY, selW, selH } = dragStart
    let newX = selX, newY = selY, newW = selW, newH = selH
    // 各手柄对应的边/角调整
    if (resizeHandle.includes('w')) { newX = selX + dx; newW = selW - dx }
    if (resizeHandle.includes('e')) { newW = selW + dx }
    if (resizeHandle.includes('n')) { newY = selY + dy; newH = selH - dy }
    if (resizeHandle.includes('s')) { newH = selH + dy }
    // 反向拖拽时翻转
    if (newW < 0) { newX += newW; newW = -newW }
    if (newH < 0) { newY += newH; newH = -newH }
    newX = Math.max(0, newX)
    newY = Math.max(0, newY)
    if (newX + newW > imgNaturalWidth.value) newW = imgNaturalWidth.value - newX
    if (newY + newH > imgNaturalHeight.value) newH = imgNaturalHeight.value - newY
    selection.value = { x: newX, y: newY, w: newW, h: newH }
  }

  function onMouseUp() {
    if (mode.value === 'drawing' && selection.value) {
      // 过小的选区视为点击，自动清除
      if (selection.value.w < 5 / zoom.value || selection.value.h < 5 / zoom.value) {
        selection.value = null
      }
    }
    mode.value = 'idle'
    stopDragListeners()
  }

  function startDragListeners() {
    window.addEventListener('mousemove', onMouseMove)
    window.addEventListener('mouseup', onMouseUp)
  }

  function stopDragListeners() {
    window.removeEventListener('mousemove', onMouseMove)
    window.removeEventListener('mouseup', onMouseUp)
  }

  function clearSelection() {
    selection.value = null
    mode.value = 'idle'
  }

  // 防御性清理：组件卸载时若仍处于拖拽中，移除残留监听
  onBeforeUnmount(() => {
    stopDragListeners()
  })

  return {
    selection,
    onCanvasMouseDown,
    onAuxClick,
    onSelectionMouseDown,
    onHandleDown,
    clearSelection,
  }
}
