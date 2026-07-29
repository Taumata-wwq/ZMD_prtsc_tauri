// 截图 Store - 管理截图会话的运行时状态
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
import { revokeBlobUrl } from '@/utils/blob'

/** 日志缓冲上限，超过后丢弃最旧的一条 */
const MAX_LOGS = 500

export const useCaptureStore = defineStore('capture', () => {
  const isRunning = ref(false)
  const isProcessing = ref(false)
  const currentSessionId = ref<number | null>(null)
  const currentSource = ref<'auto' | 'manual'>('auto')
  const progress = ref<CaptureProgress | null>(null)
  const logs = ref<CaptureLog[]>([])
  const previewImageUrl = ref<string>('')
  const capturedRegion = ref<string>('')
  const capturedScrollMode = ref<string>('')
  const { error: lastError, setError } = useAutoClearError()

  /** 事件 unlisten 句柄集合，dispose 时统一释放 */
  let unlisteners: Array<() => void> = []

  /** 初始化事件监听（幂等，重复调用不会重复注册） */
  async function init() {
    // 防止重复注册
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
        // Rust emit 的 StatusPayload 中 is_running 直接反映运行状态
        isRunning.value = e.is_running
        // 后端任务结束时（is_running=false），也重置 isProcessing
        // 避免 isProcessing 一直为 true 导致界面卡在"拼接中"状态
        // （拼接失败时不会 emit capture:preview-ready，只有 capture:status 会到达）
        if (!e.is_running) {
          isProcessing.value = false
        }
      }),
    )

    // 收到处理事件后显示加载指示器
    unlisteners.push(
      await onCaptureProcessing((count) => {
        isProcessing.value = true
        logs.value.push({
          level: 'info',
          message: `正在拼接 ${count} 张截图...`,
          timestamp: new Date().toLocaleString(),
        })
      }),
    )

    // 收到预览就绪事件后，用 convertFileSrc 将磁盘路径转为 URL
    // 规范化 Windows 路径（反斜杠→正斜杠），避免 webview asset 协议解析异常
    unlisteners.push(
      await onCapturePreviewReady((path) => {
        if (path) {
          // 规范化路径：Windows 反斜杠替换为正斜杠
          const normalizedPath = path.replace(/\\/g, '/')
          // 释放旧的 Blob URL（防止内存泄漏）
          revokeBlobUrl(previewImageUrl.value)
          previewImageUrl.value = convertFileSrc(normalizedPath)
        } else {
          // 无路径（保存失败）：回退到 getPreviewImage 字节流模式
          // 这种情况罕见，但作为后备手段保留
          api.getPreviewImage().then((buf) => {
            if (buf.byteLength > 0) {
              const blob = new Blob([buf], { type: 'image/png' })
              // 释放旧的 Blob URL
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
        // 预览就绪意味着截图流程结束
        isRunning.value = false
        isProcessing.value = false
      }),
    )

  }

  /** 启动自动截图 */
  async function start(region: string, scrollMode: string, rows?: number, cols?: number) {
    logs.value = []
    progress.value = null
    // 释放旧的 Blob URL（防止内存泄漏）
    revokeBlobUrl(previewImageUrl.value)
    previewImageUrl.value = ''
    // 锁定截图时的区域和滚动模式，导出时使用此数据
    capturedRegion.value = region
    capturedScrollMode.value = scrollMode
    isRunning.value = true
    currentSource.value = 'auto'
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

  /** 清空日志缓冲 */
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
    // state
    isRunning,
    isProcessing,
    currentSessionId,
    currentSource,
    progress,
    logs,
    previewImageUrl,
    capturedRegion,
    capturedScrollMode,
    lastError,
    // actions
    init,
    start,
    stop,
    clearLogs,
    setError,
    dispose,
  }
})
