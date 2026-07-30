/**
 * 配置 Store：区域配置与滚动模式
 *
 * 数据库只存 0次记录，其他次数由 derive_region_from_target 实时推导；
 * 大地图/自定义区域保留全部记录（grid 可编辑，不参与推导）。
 */
import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { api } from '@/api'
import type { RegionConfig, ScrollMode } from '@/types'
import { useAutoClearError } from '@/composables/useAutoClearError'

/** 大地图分类优先级（排在普通分类之后、自定义之前） */
const LARGE_MAP_PRIORITY = 998
/** 自定义分类优先级（排在最后） */
const CUSTOM_PRIORITY = 999
const DEFAULT_CATEGORY_PRIORITY = 500
const DEFAULT_SUB_MAP_PRIORITY = 999
const DEFAULT_REGION_PRIORITY = 999

const CATEGORY_ORDER: Record<string, number> = {
  '四号谷地': 0,
  '武陵': 1,
  '大地图': LARGE_MAP_PRIORITY,
  '自定义': CUSTOM_PRIORITY,
}

const REGION_ORDER: Record<string, string[]> = {
  '四号谷地': ['枢纽区', '谷地通道', '源石研究园', '供能高地'],
  '武陵': ['武陵城', '景玉谷', '首敦', '应龙关'],
  '大地图': ['枢纽区', '谷地通道', '源石研究园', '阿伯莉采石场', '矿脉园区', '供能高地',
            '武陵城', '景玉谷', '清波寨', '首敦', '藏剑谷', '试验园区', '应龙关', '北部禁区'],
}

/** 大地图子地图显示顺序 */
const SUB_MAP_ORDER: Record<string, number> = {
  '四号谷地': 0,
  '武陵': 1,
}

/**
 * 区域列表排序：先按 category 顺序，再按子区域顺序，最后按 scroll_mode。
 * 大地图先按 sub_map 排序，再按区域顺序。
 */
export function sortRegions(regions: RegionConfig[]): RegionConfig[] {
  return [...regions].sort((a, b) => {
    const catA = CATEGORY_ORDER[a.category] ?? DEFAULT_CATEGORY_PRIORITY
    const catB = CATEGORY_ORDER[b.category] ?? DEFAULT_CATEGORY_PRIORITY
    if (catA !== catB) return catA - catB

    if (a.category === '大地图' && b.category === '大地图') {
      const smA = SUB_MAP_ORDER[a.sub_map ?? ''] ?? DEFAULT_SUB_MAP_PRIORITY
      const smB = SUB_MAP_ORDER[b.sub_map ?? ''] ?? DEFAULT_SUB_MAP_PRIORITY
      if (smA !== smB) return smA - smB
    }

    const order = REGION_ORDER[a.category]
    if (order) {
      const subA = a.name.includes('-') ? a.name.split('-').slice(1).join('-') : a.name
      const subB = b.name.includes('-') ? b.name.split('-').slice(1).join('-') : b.name
      const idxA = order.indexOf(subA)
      const idxB = order.indexOf(subB)
      const finalA = idxA === -1 ? DEFAULT_REGION_PRIORITY : idxA
      const finalB = idxB === -1 ? DEFAULT_REGION_PRIORITY : idxB
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
  const { error: lastError, setError } = useAutoClearError()

  /**
   * 当前选中的区域配置（由后端实时推导）
   * 监听 currentRegionName + currentScrollModeName 变化，调用 derive_region_from_target 获取完整配置
   */
  const currentRegion = ref<RegionConfig | undefined>(undefined)

  /** 加载区域与滚动模式列表，并初始化默认选择 */
  async function load() {
    try {
      const [r, s] = await Promise.all([api.listRegions(), api.listScrollModes()])
      regions.value = sortRegions(r)
      scrollModes.value = s

      if (!currentRegionName.value && r.length > 0) {
        currentRegionName.value = r[0].name
      }

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
   * k/rate 修改后调用，使 currentRegion 反映最新推导结果。
   * 若 currentRegionName 在区域列表中不存在（如迁移后旧名称失效），自动回退到第一个可用区域。
   */
  async function refreshCurrentRegion() {
    if (!currentRegionName.value || !currentScrollModeName.value) {
      currentRegion.value = undefined
      return
    }
    if (regions.value.length > 0) {
      const exists = regions.value.some((r) => r.name === currentRegionName.value)
      if (!exists) {
        currentRegionName.value = regions.value[0].name
        return // watch 会重新触发 refreshCurrentRegion
      }
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

  watch(
    [currentRegionName, currentScrollModeName],
    () => {
      void refreshCurrentRegion()
    },
    { immediate: true },
  )

  return {
    regions,
    scrollModes,
    currentRegionName,
    currentScrollModeName,
    currentRegion,
    lastError,
    load,
    upsertRegion,
    refreshCurrentRegion,
  }
})
