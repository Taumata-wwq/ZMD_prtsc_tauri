import { ref } from 'vue'
import type { Ref } from 'vue'

/**
 * 确认弹窗 composable
 * 抽取自 HistoryView 删除确认与 SettingsView 重置确认的共同状态管理。
 *
 * 提供两种使用方式：
 * 1. 简单模式（openConfirm + handleConfirm/handleCancel）：传入消息和回调，
 *    handleConfirm 会以 isConfirming 包裹回调执行，成功后关闭弹窗。
 * 2. 直接模式（open + close + isConfirming）：调用方自行管理确认逻辑和关闭时机，
 *    适用于需要额外状态（如复选框）或自定义关闭时机（如仅成功时关闭）的场景。
 */
export function useConfirmDialog(): {
  isOpen: Ref<boolean>
  isConfirming: Ref<boolean>
  message: Ref<string>
  open: () => void
  close: () => void
  openConfirm: (msg: string, onConfirm: () => void | Promise<void>) => void
  handleConfirm: () => Promise<void>
  handleCancel: () => void
} {
  const isOpen = ref(false)
  const isConfirming = ref(false)
  const message = ref('')
  let confirmCallback: (() => void | Promise<void>) | null = null

  function open() {
    isOpen.value = true
  }

  function close() {
    isOpen.value = false
  }

  function openConfirm(msg: string, onConfirm: () => void | Promise<void>) {
    message.value = msg
    confirmCallback = onConfirm
    isOpen.value = true
  }

  async function handleConfirm() {
    const cb = confirmCallback
    if (!cb) return
    isConfirming.value = true
    try {
      await cb()
      isOpen.value = false
    } finally {
      isConfirming.value = false
      confirmCallback = null
    }
  }

  function handleCancel() {
    isOpen.value = false
    confirmCallback = null
  }

  return {
    isOpen,
    isConfirming,
    message,
    open,
    close,
    openConfirm,
    handleConfirm,
    handleCancel,
  }
}
