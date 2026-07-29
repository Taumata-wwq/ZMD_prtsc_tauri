// 配置 Store - 区域配置与滚动模式
// 数据库只存 0次记录，其他次数由 derive_region_from_target 实时推导
// 大地图/自定义区域保留全部记录（grid 可编辑，不参与推导）
import { defineStore } from 'pinia'
import { ref, watch, computed } from 'vue'
import { api } from '@/api'
import type { RegionConfig, ScrollMode } from '@/types'
import { useAutoClearError } from '@/composables/useAutoClearError'

// 排序常量

/** category 显示顺序 */
const CATEGORY_ORDER: Record<string, number> = {
  '四号谷地': 0,
  '武陵': 1,
  '大地图': 998,
  '自定义': 999,
}

/** 各 category 内子区域的显示顺序 */
const REGION_ORDER: Record<string, string[]> = {
  '四号谷地': ['枢纽区', '谷地通道', '源石研究园', '供能高地'],
  '武陵': ['武陵城', '景玉谷', '首敦', '应龙关'],
}

/**
 * 对区域列表排序：先按 category 顺序，再按子区域顺序，最后按 scroll_mode。
 * 未知 category 排在已知之后、大地图之前；未知子区域排在同组最后。
 */
export function sortRegions(regions: RegionConfig[]): RegionConfig[] {
  return [...regions].sort((a, b) => {
    const catA = CATEGORY_ORDER[a.category] ?? 500
    const catB = CATEGORY_ORDER[b.category] ?? 500
    if (catA !== catB) return catA - catB

    const order = REGION_ORDER[a.category]
    if (order) {
      const subA = a.name.includes('-') ? a.name.split('-').slice(1).join('-') : a.name
      const subB = b.name.includes('-') ? b.name.split('-').slice(1).join('-') : b.name
      const idxA = order.indexOf(subA)
      const idxB = order.indexOf(subB)
      const finalA = idxA === -1 ? 999 : idxA
      const finalB = idxB === -1 ? 999 : idxB
      if (finalA !== finalB) return finalA - finalB
    }

    return a.scroll_mode.localeCompare(b.scroll_mode)
  })
}

export const useConfigStore = defineStore('config', () => {
  /** 所有区域的 0次记录（仅用于 UI 列表展示和获取 target） */
  const regions = ref<RegionConfig[]>([])
  const scrollModes = ref<ScrollMode[]>([])
  const currentRegionName = ref<string>('')
  const currentScrollModeName = ref<string>('')
  /** 最近一次错误信息（null 表示无错误，3 秒后自动清空，供全局错误条展示） */
  const { error: lastError, setError } = useAutoClearError()

  /**
   * 当前选中的区域配置（由后端实时推导）
   *
   * 监听 currentRegionName + currentScrollModeName 变化，
   * 调用 derive_region_from_target 获取完整配置。
   * - 0次/大地图/自定义：直接返回数据库值
   * - 1-8次：用 0次 target + k + rate 推导
   */
  const currentRegion = ref<RegionConfig | undefined>(undefined)

  /** 当前选中的滚动模式对象 */
  const currentScrollMode = computed<ScrollMode | undefined>(() => {
    return scrollModes.value.find((s) => s.name === currentScrollModeName.value)
  })

  /** 加载区域与滚动模式列表，并初始化默认选择 */
  async function load() {
    try {
      const [r, s] = await Promise.all([api.listRegions(), api.listScrollModes()])
      regions.value = sortRegions(r)
      scrollModes.value = s

      // 设置默认区域（若未选择）
      if (!currentRegionName.value && r.length > 0) {
        currentRegionName.value = r[0].name
      }

      // 设置默认滚动模式（优先 is_default，否则取第一个）
      if (!currentScrollModeName.value) {
        const defaultScroll = s.find((m) => m.is_default) || s[0]
        if (defaultScroll) {
          currentScrollModeName.value = defaultScroll.name
        }
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      setError(msg)
      throw e
    }
  }

  /** 新增或更新区域配置（upsert） */
  async function upsertRegion(config: RegionConfig) {
    try {
      await api.upsertRegion(config)
      await load()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }

  /**
   * 重新推导当前区域配置
   *
   * 在 k 值/rate 值修改后调用，使 currentRegion 反映最新推导结果。
   */
  async function refreshCurrentRegion() {
    if (!currentRegionName.value || !currentScrollModeName.value) {
      currentRegion.value = undefined
      return
    }
    try {
      const r = await api.deriveRegionFromTarget(
        currentRegionName.value,
        '16:9',
        currentScrollModeName.value,
      )
      currentRegion.value = r ?? undefined
    } catch (e) {
      console.error('[configStore] deriveRegionFromTarget 失败:', e)
      currentRegion.value = undefined
    }
  }

  // 监听区域或滚动次数变化，实时推导 currentRegion
  watch(
    [currentRegionName, currentScrollModeName],
    () => {
      void refreshCurrentRegion()
    },
    { immediate: true },
  )

  return {
    // state
    regions,
    scrollModes,
    currentRegionName,
    currentScrollModeName,
    currentRegion,
    lastError,
    // getters
    currentScrollMode,
    // actions
    load,
    upsertRegion,
    refreshCurrentRegion,
  }
})
