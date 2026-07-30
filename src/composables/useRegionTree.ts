import { computed } from 'vue'
import type { Ref } from 'vue'
import type { RegionConfig } from '@/types'

// 客户端逻辑视口尺寸（与后端 derive_all_counts 保持一致）
const CLIENT_W = 1920
const CLIENT_H = 1080

/** 基建区域分组（同名不同 scroll_mode 合并为一项） */
export interface RegionGroup {
  name: string
  category: string
  representative: RegionConfig
  targetW: number
  targetH: number
}

/** 基建分类（一级节点） */
export interface TreeCategory {
  name: string
  items: RegionGroup[]
}

/** 大地图子地图（一级节点下的二级项） */
export interface LargeMapSub {
  name: string
  count: number
}

export interface LargeMapArea {
  name: string
  region: RegionConfig
}

/** 由 grid/overlap 反推区域的目标宽高（target 优先，缺失时由 capture_region + grid 推算） */
export function calcTarget(region: RegionConfig): { w: number; h: number } {
  if (region.target_w > 0 && region.target_h > 0) {
    return { w: region.target_w, h: region.target_h }
  }
  const imgW = Math.round(CLIENT_W * region.capture_region_x)
  const imgH = Math.round(CLIENT_H * region.capture_region_y)
  const ovlpPxX = Math.trunc(imgW * region.overlap_x)
  const ovlpPxY = Math.trunc(imgH * region.overlap_y)
  const stepX = imgW - ovlpPxX
  const stepY = imgH - ovlpPxY
  return { w: stepX * region.grid_cols + ovlpPxX, h: stepY * region.grid_rows + ovlpPxY }
}

/**
 * 树形数据计算：基建分类树 + 大地图子地图列表 + 当前子地图区域
 * 仅包含纯计算逻辑，UI 状态（展开集合、选中项）由调用方维护。
 */
export function useRegionTree(
  regions: Ref<RegionConfig[]>,
  selectedLargeMapSub: Ref<string | null>,
) {
  /** 基建分类树（排除 "自定义" 与 "大地图"） */
  const baseTree = computed<TreeCategory[]>(() => {
    const nameMap = new Map<string, RegionGroup>()
    regions.value.forEach((r) => {
      if (r.category === '自定义' || r.category === '大地图') return
      if (!nameMap.has(r.name)) {
        nameMap.set(r.name, {
          name: r.name,
          category: r.category,
          representative: r,
          targetW: 0,
          targetH: 0,
        })
      }
      if (r.scroll_mode === '0次') {
        nameMap.get(r.name)!.representative = r
      }
    })

    const catMap = new Map<string, RegionGroup[]>()
    nameMap.forEach((group) => {
      const target = calcTarget(group.representative)
      group.targetW = target.w
      group.targetH = target.h
      const cat = group.category || '未分类'
      if (!catMap.has(cat)) catMap.set(cat, [])
      catMap.get(cat)!.push(group)
    })

    const result: TreeCategory[] = []
    catMap.forEach((items, name) => {
      items.sort((a, b) => a.name.localeCompare(b.name, 'zh'))
      result.push({ name, items })
    })
    result.sort((a, b) => {
      if (a.name === '四号谷地') return -1
      if (b.name === '四号谷地') return 1
      return a.name.localeCompare(b.name, 'zh')
    })
    return result
  })

  const largeMapSubMaps = computed<LargeMapSub[]>(() => {
    const map = new Map<string, number>()
    regions.value.forEach((r) => {
      if (r.category !== '大地图' || !r.sub_map) return
      map.set(r.sub_map, (map.get(r.sub_map) ?? 0) + 1)
    })
    const result = Array.from(map.entries()).map(([name, count]) => ({ name, count }))
    result.sort((a, b) => {
      if (a.name === '四号谷地') return -1
      if (b.name === '四号谷地') return 1
      return a.name.localeCompare(b.name, 'zh')
    })
    return result
  })

  const totalLargeMapCount = computed(() =>
    largeMapSubMaps.value.reduce((sum, sm) => sum + sm.count, 0),
  )

  const currentSubMapAreas = computed<LargeMapArea[]>(() => {
    if (!selectedLargeMapSub.value) return []
    return regions.value
      .filter((r) => r.category === '大地图' && r.sub_map === selectedLargeMapSub.value)
      .map((r) => ({ name: r.name, region: r }))
      .sort((a, b) => a.name.localeCompare(b.name, 'zh'))
  })

  return {
    baseTree,
    largeMapSubMaps,
    totalLargeMapCount,
    currentSubMapAreas,
  }
}
