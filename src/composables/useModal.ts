import { ref, nextTick, watch, type Ref } from 'vue'
import { useI18n } from '@/composables/useI18n'

/**
 * 全局模态弹窗 composable
 * 提供 confirmDialog / promptDialog 两个 Promise 式 API，替代 web 自带 confirm/prompt。
 * 状态在模块级单例 state 中，ModalDialog.vue 组件负责渲染并绑定 inputEl。
 * 与 useConfirmDialog（组件内局部确认弹窗）互不冲突。
 */

interface HiddenModalState {
  visible: false
}

interface BaseModalState {
  visible: true
  title: string
  message: string
  input: string
  placeholder: string
  confirmText: string
  cancelText: string
  danger: boolean
}

export interface ConfirmModalState extends BaseModalState {
  mode: 'confirm'
  resolve: ((v: boolean) => void) | null
}

export interface PromptModalState extends BaseModalState {
  mode: 'prompt'
  resolve: ((v: string | null) => void) | null
}

export type ModalState = HiddenModalState | ConfirmModalState | PromptModalState

export interface ConfirmDialogOptions {
  title: string
  message?: string
  confirmText?: string
  cancelText?: string
  danger?: boolean
}

export interface PromptDialogOptions {
  title: string
  message?: string
  defaultValue?: string
  placeholder?: string
  confirmText?: string
  cancelText?: string
}

export const state = ref<ModalState>({ visible: false })

export const inputEl: Ref<HTMLInputElement | null> = ref(null)

function onKeydown(e: KeyboardEvent) {
  if (!state.value.visible) return
  if (e.key === 'Escape') cancel()
}

export function confirm() {
  const s = state.value
  if (!s.visible) return
  if (s.mode === 'confirm') {
    s.resolve?.(true)
  } else {
    s.resolve?.(s.input)
  }
  state.value = { visible: false }
}

export function cancel() {
  const s = state.value
  if (!s.visible) return
  if (s.mode === 'confirm') {
    s.resolve?.(false)
  } else {
    s.resolve?.(null)
  }
  state.value = { visible: false }
}

export function confirmDialog(options: ConfirmDialogOptions): Promise<boolean> {
  const { t } = useI18n()
  const next: ConfirmModalState = {
    visible: true,
    mode: 'confirm',
    title: options.title,
    message: options.message ?? '',
    input: '',
    placeholder: '',
    confirmText: options.confirmText ?? t('common.confirm'),
    cancelText: options.cancelText ?? t('common.cancel'),
    danger: options.danger ?? false,
    resolve: null,
  }
  state.value = next
  return new Promise<boolean>((resolve) => {
    next.resolve = resolve
  })
}

export function promptDialog(options: PromptDialogOptions): Promise<string | null> {
  const { t } = useI18n()
  const next: PromptModalState = {
    visible: true,
    mode: 'prompt',
    title: options.title,
    message: options.message ?? '',
    input: options.defaultValue ?? '',
    placeholder: options.placeholder ?? '',
    confirmText: options.confirmText ?? t('common.confirm'),
    cancelText: options.cancelText ?? t('common.cancel'),
    danger: false,
    resolve: null,
  }
  state.value = next
  nextTick(() => inputEl.value?.focus())
  return new Promise<string | null>((resolve) => {
    next.resolve = resolve
  })
}

watch(
  () => state.value.visible,
  (v) => {
    if (v) {
      const s = state.value
      if (s.visible && s.mode === 'prompt') {
        nextTick(() => inputEl.value?.focus())
      }
      document.addEventListener('keydown', onKeydown)
    } else {
      document.removeEventListener('keydown', onKeydown)
    }
  },
)

export function useModal() {
  return {
    state,
    inputEl,
    confirm,
    cancel,
  }
}
