import { defineStore } from 'pinia'
import { ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import {
  api,
  onCaptureProgress,
  onCaptureLog,
  onCaptureStatus,
  onCapturePreviewReady,
  onCaptureProcessing,
} from '@/api'
import type {
  CaptureProgress,
  CaptureLog,
} from '@/types'
import { useAutoClearError } from '@/composables/useAutoClearError'
import { useI18n } from '@/composables/useI18n'
import { revokeBlobUrl } from '@/utils/blob'
import { formatDateTime } from '@/utils/datetime'
import { useHistoryStore } from '@/stores/history.store'

/** 日志缓冲上限，超过后丢弃最旧的一条 */
const MAX_LOGS = 500

export const useCaptureStore = defineStore('capture', () => {
  const isRunning = ref(false)
  const isProcessing = ref(false)
  const currentSessionId = ref<number | null>(null)
  const progress = ref<CaptureProgress | null>(null)
  const logs = ref<CaptureLog[]>([])
  const previewImageUrl = ref<string>('')
  const capturedRegion = ref<string>('')
  const capturedScrollMode = ref<string>('')
  const { error: lastError, setError } = useAutoClearError()
  const { t } = useI18n()

  /** 事件 unlisten 句柄集合，dispose 时统一释放 */
  let unlisteners: Array<() => void> = []

  /** 初始化事件监听（幂等，重复调用会先释放旧监听） */
  async function init() {
    if (unlisteners.length > 0) {
      dispose()
    }

    unlisteners.push(
      await onCaptureProgress((e) => {
        progress.value = e
      }),
    )

    unlisteners.push(
      await onCaptureLog((e) => {
        logs.value.push(e)
        if (logs.value.length > MAX_LOGS) {
          logs.value.shift()
        }
      }),
    )

    unlisteners.push(
      await onCaptureStatus((e) => {
        isRunning.value = e.is_running
        // 拼接失败时后端不会 emit preview-ready，仅 status 到达，需在此重置 isProcessing 避免界面卡死
        if (!e.is_running) {
          isProcessing.value = false
        }
      }),
    )

    unlisteners.push(
      await onCaptureProcessing((count) => {
        isProcessing.value = true
        logs.value.push({
          level: 'info',
          message: t('capture.stitching', { count }),
          timestamp: formatDateTime(new Date(), 'zh'),
        })
      }),
    )

    unlisteners.push(
      await onCapturePreviewReady(async (path) => {
        if (path) {
          // 规范化 Windows 路径分隔符，避免 webview asset 协议解析异常
          const normalizedPath = path.replace(/\\/g, '/')
          revokeBlobUrl(previewImageUrl.value)
          previewImageUrl.value = convertFileSrc(normalizedPath)
        } else {
          // 无路径（保存失败）时回退到字节流模式
          api.getPreviewImage().then((buf) => {
            if (buf.byteLength > 0) {
              const blob = new Blob([buf], { type: 'image/png' })
              revokeBlobUrl(previewImageUrl.value)
              previewImageUrl.value = URL.createObjectURL(blob)
            } else {
              previewImageUrl.value = ''
            }
          }).catch((e) => {
            console.error('[capture.store] getPreviewImage 后备拉取失败:', e)
            previewImageUrl.value = ''
          })
        }
        isRunning.value = false
        isProcessing.value = false
        // 截图完成后刷新历史记录；失败不影响截图流程
        try {
          await useHistoryStore().load()
        } catch {
          // ignore
        }
      }),
    )

  }

  /** 启动自动截图 */
  async function start(region: string, scrollMode: string, rows?: number, cols?: number) {
    logs.value = []
    progress.value = null
    revokeBlobUrl(previewImageUrl.value)
    previewImageUrl.value = ''
    // 锁定截图时的区域和滚动模式，导出时使用
    capturedRegion.value = region
    capturedScrollMode.value = scrollMode
    isRunning.value = true
    try {
      const sid = await api.startCapture(region, scrollMode, rows, cols)
      currentSessionId.value = sid
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }

  /** 停止截图（在下一个循环检查点退出） */
  async function stop() {
    try {
      await api.stopCapture()
      isRunning.value = false
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }

  /** 从历史记录加载原图到编辑区域（用于重新编辑） */
  async function loadFromSession(session: {
    id?: number | null
    original_path?: string | null
    region?: string | null
    scroll_mode?: string | null
  }) {
    if (!session.original_path) {
      setError('该历史记录没有原图路径')
      return
    }
    revokeBlobUrl(previewImageUrl.value)
    const normalizedPath = session.original_path.replace(/\\/g, '/')
    previewImageUrl.value = convertFileSrc(normalizedPath)
    // 设置后端 preview_path，使导出时 getSourcePath 能获取正确路径
    try {
      await api.setPreviewPath(session.original_path)
    } catch (e) {
      console.error('[capture.store] setPreviewPath 失败:', e)
    }
    capturedRegion.value = session.region ?? ''
    capturedScrollMode.value = session.scroll_mode ?? ''
    currentSessionId.value = session.id ?? null
    isRunning.value = false
    isProcessing.value = false
  }

  function clearLogs() {
    logs.value = []
  }

  /** 释放所有事件监听器（组件 unmount 时调用） */
  function dispose() {
    unlisteners.forEach((fn) => {
      try {
        fn()
      } catch {
        // 忽略已释放的监听器
      }
    })
    unlisteners = []
  }

  return {
    isRunning,
    isProcessing,
    currentSessionId,
    progress,
    logs,
    previewImageUrl,
    capturedRegion,
    capturedScrollMode,
    lastError,
    init,
    start,
    stop,
    loadFromSession,
    clearLogs,
    setError,
    dispose,
  }
})
