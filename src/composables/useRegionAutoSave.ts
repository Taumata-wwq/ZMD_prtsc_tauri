import { ref, watch } from 'vue'
import type { Ref } from 'vue'
import { api, deriveAllCounts } from '@/api'
import { useSettingsStore } from '@/stores/settings.store'
import type { RegionConfig, AllCountsResult } from '@/types'

// 客户端逻辑视口尺寸（与后端 derive_all_counts 保持一致）
const CLIENT_W = 1920
const CLIENT_H = 1080

export interface RegionAutoSaveDeps {
  /** 当前选中的基建区域名（仅用于判断是否需要触发自动保存） */
  selectedBaseName: Ref<string | null>
  /** 当前选中的基建区域配置（用于保存时取原始字段） */
  selectedBaseRegion: Ref<RegionConfig | null>
  editTargetW: Ref<number>
  editTargetH: Ref<number>
  /** 统一的刷新回调（执行后刷新本地列表 + store） */
  withRefresh: <T>(fn: () => Promise<T>) => Promise<T | undefined>
}

/**
 * 基建区域自动保存：watch target_w/target_h 触发推导 + 延迟 800ms 落库
 * - skipAutoSave：主动切换选中项时跳过本次自动保存
 * - flushPendingSave：卸载/切换面板时强制落库未保存的修改
 */
export function useRegionAutoSave(deps: RegionAutoSaveDeps) {
  const { selectedBaseName, selectedBaseRegion, editTargetW, editTargetH, withRefresh } = deps
  const settingsStore = useSettingsStore()

  const allCountsResult = ref<AllCountsResult | null>(null)
  let skipAutoSave = false
  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null

  /** 将当前推导结果落库（写入 0次 记录） */
  async function saveExisting(allCounts: AllCountsResult | null) {
    if (!selectedBaseRegion.value || !allCounts) return
    const original = selectedBaseRegion.value
    const derived0 = allCounts.counts[0]
    await withRefresh(async () => {
      await api.upsertRegion({
        ...original,
        scroll_mode: '0次',
        grid_rows: derived0.actual_rows,
        grid_cols: derived0.actual_cols,
        overlap_x: derived0.overlap_x,
        overlap_y: derived0.overlap_y,
        drag_x: derived0.drag_x,
        drag_y: derived0.drag_y,
        target_w: editTargetW.value,
        target_h: editTargetH.value,
        updated_at: new Date().toISOString(),
      })
    })
  }

  /** 排定 800ms 延迟保存，覆盖任何在途定时器 */
  function scheduleAutoSave(allCounts: AllCountsResult | null) {
    if (autoSaveTimer) clearTimeout(autoSaveTimer)
    autoSaveTimer = setTimeout(async () => {
      autoSaveTimer = null
      await saveExisting(allCounts)
    }, 800)
  }

  /** 立即落库在途修改（卸载/切面板时调用） */
  function flushPendingSave() {
    if (!autoSaveTimer) return
    clearTimeout(autoSaveTimer)
    autoSaveTimer = null
    const pending = allCountsResult.value
    if (pending && selectedBaseRegion.value) {
      void saveExisting(pending)
    }
  }

  /** 标记下一次 watch 触发时跳过自动保存（用于程序化赋值 target 后的 rederive） */
  function setSkipAutoSave() {
    skipAutoSave = true
  }

  /** 同步重新推导 allCountsResult（不经过 watch，不触发自动保存） */
  async function rederive() {
    if (editTargetW.value > 0 && editTargetH.value > 0) {
      const overlapMin = settingsStore.settings?.overlap_min ?? 0.0
      const overlapMax = settingsStore.settings?.overlap_max ?? 0.5
      try {
        allCountsResult.value = await deriveAllCounts(
          CLIENT_W, CLIENT_H, editTargetW.value, editTargetH.value, overlapMin, overlapMax,
        )
      } catch {
        allCountsResult.value = null
      }
    }
  }

  // 监听 target 变化：重新推导 + 触发自动保存（除非 skipAutoSave）
  watch(
    [editTargetW, editTargetH],
    async ([w, h]) => {
      if (w <= 0 || h <= 0) {
        allCountsResult.value = null
        return
      }
      const wasSkip = skipAutoSave
      skipAutoSave = false
      try {
        const overlapMin = settingsStore.settings?.overlap_min ?? 0.0
        const overlapMax = settingsStore.settings?.overlap_max ?? 0.5
        allCountsResult.value = await deriveAllCounts(CLIENT_W, CLIENT_H, w, h, overlapMin, overlapMax)
        if (!wasSkip && selectedBaseName.value && selectedBaseRegion.value) {
          scheduleAutoSave(allCountsResult.value)
        }
      } catch (e) {
        console.error('[DataManageView] deriveAllCounts 失败:', e)
        allCountsResult.value = null
      }
    },
  )

  return {
    allCountsResult,
    scheduleAutoSave,
    flushPendingSave,
    setSkipAutoSave,
    rederive,
  }
}
