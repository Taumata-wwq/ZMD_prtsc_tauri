import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/api'
import type { CaptureSession } from '@/types'
import { useAutoClearError } from '@/composables/useAutoClearError'

export const useHistoryStore = defineStore('history', () => {
  const sessions = ref<CaptureSession[]>([])
  const loading = ref(false)
  const { error, setError } = useAutoClearError()

  async function load() {
    loading.value = true
    try {
      sessions.value = await api.listSessions()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
      throw e
    } finally {
      loading.value = false
    }
  }

  /** 重新加载历史记录列表，错误已写入 error 状态，不再向外抛出 */
  async function refresh() {
    try {
      await load()
    } catch {
      // ignore
    }
  }

  async function deleteSession(
    id: number,
    deleteOriginal?: boolean,
    deleteScreenshot?: boolean,
  ) {
    await api.deleteSession(id, deleteOriginal ?? false, deleteScreenshot ?? false)
    await refresh()
  }

  return {
    sessions,
    loading,
    error,
    load,
    refresh,
    deleteSession,
  }
})
