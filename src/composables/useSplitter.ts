import { ref, onMounted, onBeforeUnmount, onDeactivated } from 'vue'
import type { Ref } from 'vue'

interface SplitterOptions {
  storageKey: string
  defaultWidth: number
  minWidth: number
  maxWidth: number
  /** 分栏容器选择器，用于在 mousemove 中通过 getBoundingClientRect 计算相对位置 */
  containerSelector: string
}

/**
 * 左右分栏拖拽逻辑 composable：拖拽时实时更新宽度，松手后持久化到 localStorage
 */
export function useSplitter(options: SplitterOptions): {
  width: Ref<number>
  isDragging: Ref<boolean>
  loadWidth: () => void
  onMouseDown: (e: MouseEvent) => void
  onMouseMove: (e: MouseEvent) => void
  onMouseUp: () => void
} {
  const { storageKey, defaultWidth, minWidth, maxWidth, containerSelector } = options

  const width = ref<number>(defaultWidth)
  const isDragging = ref<boolean>(false)

  /** 从 localStorage 恢复上次位置，失败时使用默认值 */
  function loadWidth() {
    try {
      const stored = localStorage.getItem(storageKey)
      if (stored) {
        const n = parseInt(stored, 10)
        if (!Number.isNaN(n) && n >= minWidth && n <= maxWidth) {
          width.value = n
        }
      }
    } catch {
      // localStorage 不可用时使用默认值
    }
  }

  function onMouseMove(e: MouseEvent) {
    if (!isDragging.value) return
    const container = document.querySelector(containerSelector) as HTMLElement | null
    if (!container) return
    const rect = container.getBoundingClientRect()
    const newWidth = e.clientX - rect.left
    width.value = Math.min(maxWidth, Math.max(minWidth, newWidth))
  }

  function onMouseUp() {
    if (!isDragging.value) return
    isDragging.value = false
    document.body.style.userSelect = ''
    document.body.style.cursor = ''
    window.removeEventListener('mousemove', onMouseMove)
    window.removeEventListener('mouseup', onMouseUp)
    try {
      localStorage.setItem(storageKey, String(width.value))
    } catch {
      // localStorage 不可用时忽略
    }
  }

  function onMouseDown(e: MouseEvent) {
    if (e.button !== 0) return
    isDragging.value = true
    // 拖拽期间禁用文本选择，避免选中配置面板内容
    document.body.style.userSelect = 'none'
    document.body.style.cursor = 'col-resize'
    window.addEventListener('mousemove', onMouseMove)
    window.addEventListener('mouseup', onMouseUp)
    e.preventDefault()
  }

  onMounted(() => {
    loadWidth()
  })

  // 防御性清理：组件在拖拽中被卸载时移除残留监听
  onBeforeUnmount(() => {
    window.removeEventListener('mousemove', onMouseMove)
    window.removeEventListener('mouseup', onMouseUp)
    document.body.style.userSelect = ''
    document.body.style.cursor = ''
  })

  // keep-alive 切换时清理拖拽状态，避免鼠标事件泄漏到其他视图
  onDeactivated(() => {
    window.removeEventListener('mousemove', onMouseMove)
    window.removeEventListener('mouseup', onMouseUp)
    document.body.style.userSelect = ''
    document.body.style.cursor = ''
  })

  return {
    width,
    isDragging,
    loadWidth,
    onMouseDown,
    onMouseMove,
    onMouseUp,
  }
}
