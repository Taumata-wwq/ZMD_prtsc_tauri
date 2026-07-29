// 历史记录 Store
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/api'
import type { CaptureSession } from '@/types'

export const useHistoryStore = defineStore('history', () => {
  const sessions = ref<CaptureSession[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  /**
   * 加载历史会话
   *
   * @param limit 会话条数上限（undefined 使用 Rust 默认值）
   */
  async function load(limit?: number) {
    loading.value = true
    error.value = null
    try {
      sessions.value = await api.listSessions(limit)
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  /** 清空所有历史记录 */
  async function clear() {
    loading.value = true
    error.value = null
    try {
      await api.clearHistory()
      sessions.value = []
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    // state
    sessions,
    loading,
    error,
    // actions
    load,
    clear,
  }
})
