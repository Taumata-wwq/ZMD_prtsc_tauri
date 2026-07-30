<template>
  <div class="config-panel">
    <div v-if="!settingsStore.settings" class="loading">{{ t('common.loading') }}</div>

    <template v-else>
      <section class="layer basic-layer">
        <div class="field-group">
          <label class="field-label">{{ t('capture.region') }}</label>
          <div class="region-pair">
            <select
              class="field-select"
              :value="currentCategory"
              :disabled="captureStore.isRunning"
              @change="onCategoryChange"
            >
              <option v-for="cat in categories" :key="cat" :value="cat">
                {{ cat }}
              </option>
            </select>
            <!-- 大地图：子地图选择 -->
            <select
              v-if="currentCategory === '大地图'"
              class="field-select"
              :value="currentLargeMapSubMap"
              :disabled="captureStore.isRunning"
              @change="onLargeMapSubMapChange"
            >
              <option v-for="sm in largeMapSubMaps" :key="sm" :value="sm">
                {{ sm }}
              </option>
            </select>
            <!-- 基建：子名选择 -->
            <select
              v-else
              class="field-select"
              :value="currentSubName"
              :disabled="captureStore.isRunning || !hasSubNames"
              @change="onSubNameChange"
            >
              <option v-for="sub in subNames" :key="sub" :value="sub">
                {{ sub }}
              </option>
            </select>
          </div>
        </div>

        <!-- 大地图：子区域选择（自定义时 disabled） -->
        <div v-if="currentCategory === '大地图'" class="field-group">
          <label class="field-label">{{ t('capture.subRegion') }}</label>
          <select
            class="field-select"
            :value="currentLargeMapArea"
            :disabled="captureStore.isRunning || isLargeMapCustom"
            @change="onLargeMapAreaChange"
          >
            <option v-if="isLargeMapCustom" value="">{{ t('capture.customRegion') }}</option>
            <option v-else v-for="area in largeMapAreas" :key="area" :value="area">
              {{ area }}
            </option>
          </select>
        </div>

        <!-- 滚动模式（仅基建模式显示） -->
        <div v-else class="field-group">
          <label class="field-label">{{ t('capture.scrollMode') }}</label>
          <select
            class="field-select"
            :value="configStore.currentScrollModeName"
            :disabled="captureStore.isRunning || isStaticMode"
            @change="onScrollModeSelect"
          >
            <option v-for="mode in configStore.scrollModes" :key="mode.name" :value="mode.name">
              {{ mode.name }}
            </option>
          </select>
        </div>

        <!-- 网格大小（自定义/大地图自定义可编辑，其他只读） -->
        <div class="field-group">
          <label class="field-label">{{ t('capture.gridSize') }}</label>
          <div class="number-pair">
            <input
              type="number"
              class="field-input"
              :min="1"
              :max="100"
              :value="gridRows"
              :readonly="!isGridEditable"
              :disabled="captureStore.isRunning && !isGridEditable"
              @input="onGridRowsInput"
            />
            <span class="pair-sep">×</span>
            <input
              type="number"
              class="field-input"
              :min="1"
              :max="100"
              :value="gridCols"
              :readonly="!isGridEditable"
              :disabled="captureStore.isRunning && !isGridEditable"
              @input="onGridColsInput"
            />
          </div>
        </div>

        <div class="field-group">
          <label class="field-label">{{ t('capture.outputFormat') }}</label>
          <select
            class="field-select"
            :value="settingsStore.settings?.output_format ?? 'JPG'"
            :disabled="captureStore.isRunning"
            @change="onFormatChange(($event.target as HTMLSelectElement).value as 'JPG' | 'PNG')"
          >
            <option value="JPG">JPG</option>
            <option value="PNG">PNG</option>
          </select>
        </div>
        <div v-if="settingsStore.settings?.output_format === 'JPG'" class="field-group">
          <label class="field-label">{{ t('capture.quality') }}（{{ settingsStore.settings?.jpg_quality ?? 95 }}）</label>
          <input
            type="range"
            class="quality-slider"
            min="1"
            max="100"
            step="1"
            :value="settingsStore.settings?.jpg_quality ?? 95"
            :disabled="captureStore.isRunning"
            @input="onJpgQualityChange(parseInt(($event.target as HTMLInputElement).value, 10))"
          />
        </div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useConfigStore } from '@/stores/config.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useCaptureStore } from '@/stores/capture.store'
import { useI18n } from '@/composables/useI18n'
import type { RegionConfig, AppSettingKey, AppSettings } from '@/types'
import { buildRegionName, stripCategoryPrefix, getCategoryPrefix } from '@/utils/regionName'

const configStore = useConfigStore()
const settingsStore = useSettingsStore()
const captureStore = useCaptureStore()
const { t } = useI18n()

const regionsByCategory = computed<Record<string, string[]>>(() => {
  const map: Record<string, string[]> = {}
  for (const r of configStore.regions) {
    if (!map[r.category]) map[r.category] = []
    if (!map[r.category].includes(r.name)) {
      map[r.category].push(r.name)
    }
  }
  return map
})

/** 可选主类列表（保持插入顺序） */
const categories = computed<string[]>(() => {
  const seen = new Set<string>()
  const list: string[] = []
  for (const r of configStore.regions) {
    if (!seen.has(r.category)) {
      seen.add(r.category)
      list.push(r.category)
    }
  }
  return list
})

/** 当前主类：从 currentRegionName 解析，大地图子区域通过 regions 查找 */
const currentCategory = computed<string>(() => {
  const name = configStore.currentRegionName
  if (!name) return categories.value[0] ?? ''
  // 大地图子区域 name 不带前缀，如 "枢纽区"
  const region = configStore.regions.find((r) => r.name === name && r.category === '大地图')
  if (region) return '大地图'
  if (name === '自定义') return '自定义'
  return getCategoryPrefix(name) ?? name
})

/** 当前主类下的子名列表（去除 category 前缀） */
const subNames = computed<string[]>(() => {
  const cat = currentCategory.value
  if (!cat || cat === '自定义') return []
  const names = regionsByCategory.value[cat] ?? []
  return names
    .map((n) => stripCategoryPrefix(n))
    .filter((v, i, arr) => arr.indexOf(v) === i)
})

const hasSubNames = computed<boolean>(() => subNames.value.length > 0)

const currentSubName = computed<string>(() => {
  const name = configStore.currentRegionName
  if (!name) return ''
  if (name === '自定义') return ''
  return getCategoryPrefix(name) !== null ? stripCategoryPrefix(name) : ''
})

/** 大地图子地图列表（四号谷地 / 武陵 / 自定义） */
const largeMapSubMaps = computed<string[]>(() => {
  const set = new Set<string>()
  for (const r of configStore.regions) {
    if (r.category === '大地图' && r.sub_map) {
      set.add(r.sub_map)
    }
  }
  const list = [...set]
  // 末尾追加"自定义"选项
  if (!list.includes('自定义')) list.push('自定义')
  return list
})

const currentLargeMapSubMap = computed<string>(() => {
  if (settingsStore.settings?.last_large_map_custom) return '自定义'
  const name = configStore.currentRegionName
  const region = configStore.regions.find((r) => r.name === name && r.category === '大地图')
  return region?.sub_map ?? ''
})

const isLargeMapCustom = computed<boolean>(() => {
  return currentCategory.value === '大地图' && !!settingsStore.settings?.last_large_map_custom
})

const largeMapAreas = computed<string[]>(() => {
  const sm = currentLargeMapSubMap.value
  if (!sm) return []
  return configStore.regions
    .filter((r) => r.category === '大地图' && r.sub_map === sm)
    .map((r) => r.name)
})

const currentLargeMapArea = computed<string>(() => {
  if (currentCategory.value !== '大地图') return ''
  if (isLargeMapCustom.value) return ''
  return configStore.currentRegionName
})

/** 大地图/自定义：固定滚动次数，不可切换 */
const isStaticMode = computed<boolean>(() => {
  const cat = currentCategory.value
  return cat === '自定义' || cat === '大地图'
})

/** 网格是否可编辑：自定义 / 大地图自定义 */
const isGridEditable = computed<boolean>(() => {
  return currentCategory.value === '自定义' || isLargeMapCustom.value
})

/** 持久化 setting 项，统一捕获错误 */
async function persistSetting<K extends AppSettingKey>(key: K, value: AppSettings[K]) {
  try {
    await settingsStore.update(key, value)
  } catch (e) {
    console.error(`[ConfigPanel] 更新 ${key} 失败:`, e)
  }
}

async function onCategoryChange(event: Event) {
  const target = event.target as HTMLSelectElement
  const category = target.value

  if (category === '大地图') {
    await selectLargeMapFirstArea()
    configStore.currentScrollModeName = '0次'
    await persistSetting('last_scroll_mode', '0次')
    return
  }

  // 基建/自定义：选第一个子名
  const subs = (regionsByCategory.value[category] ?? [])
    .map((n) => {
      if (category === '自定义') return ''
      return stripCategoryPrefix(n)
    })
    .filter((v, i, arr) => v && arr.indexOf(v) === i)
  const fullName = buildRegionName(category, subs[0] ?? '')
  configStore.currentRegionName = fullName
  await persistSetting('last_region', fullName)
  // 自定义固定滚动模式为0次
  if (category === '自定义') {
    configStore.currentScrollModeName = '0次'
    await persistSetting('last_scroll_mode', '0次')
  }
}

/** 大地图模式：选第一个子地图的第一个区域；若已是大地图自定义模式且当前区域合法则保留 */
async function selectLargeMapFirstArea() {
  const isCustom = !!settingsStore.settings?.last_large_map_custom
  const currentName = configStore.currentRegionName
  if (isCustom) {
    const isValid = configStore.regions.some(
      (r) => r.name === currentName && r.category === '大地图',
    )
    if (isValid) return
  }
  const firstSubMap = largeMapSubMaps.value.find((s) => s !== '自定义') ?? ''
  const area = configStore.regions.find(
    (r) => r.category === '大地图' && r.sub_map === firstSubMap,
  )?.name
  if (area) {
    configStore.currentRegionName = area
    await persistSetting('last_region', area)
  }
}

async function onSubNameChange(event: Event) {
  const target = event.target as HTMLSelectElement
  const sub = target.value
  const fullName = buildRegionName(currentCategory.value, sub)
  configStore.currentRegionName = fullName
  await persistSetting('last_region', fullName)
}

/** 大地图子地图切换：选该子地图下第一个区域，"自定义"则进入大地图自定义模式 */
async function onLargeMapSubMapChange(event: Event) {
  const target = event.target as HTMLSelectElement
  const subMap = target.value
  if (subMap === '自定义') {
    await persistSetting('last_large_map_custom', true)
    return
  }
  await persistSetting('last_large_map_custom', false)
  const area = configStore.regions.find(
    (r) => r.category === '大地图' && r.sub_map === subMap,
  )?.name
  if (area) {
    configStore.currentRegionName = area
    await persistSetting('last_region', area)
  }
}

async function onLargeMapAreaChange(event: Event) {
  const target = event.target as HTMLSelectElement
  const area = target.value
  configStore.currentRegionName = area
  await persistSetting('last_region', area)
}

const localGridRows = ref<number>(0)
const localGridCols = ref<number>(0)

const gridRows = computed<number>(() => {
  if (isGridEditable.value) {
    // 大地图自定义：使用 last_rows，默认 2
    if (isLargeMapCustom.value) {
      return localGridRows.value || (settingsStore.settings?.last_rows ?? 2)
    }
    return localGridRows.value || (settingsStore.settings?.last_rows ?? 0)
  }
  return configStore.currentRegion?.grid_rows ?? 0
})

const gridCols = computed<number>(() => {
  if (isGridEditable.value) {
    // 大地图自定义：使用 last_cols，默认 2
    if (isLargeMapCustom.value) {
      return localGridCols.value || (settingsStore.settings?.last_cols ?? 2)
    }
    return localGridCols.value || (settingsStore.settings?.last_cols ?? 0)
  }
  return configStore.currentRegion?.grid_cols ?? 0
})

// 监听 currentRegion 变化，重置本地编辑值（仅基建自定义时跟随 currentRegion）
watch(
  () => configStore.currentRegion,
  (r) => {
    if (r && isGridEditable.value && !isLargeMapCustom.value) {
      localGridRows.value = r.grid_rows
      localGridCols.value = r.grid_cols
    } else if (!isLargeMapCustom.value) {
      localGridRows.value = 0
      localGridCols.value = 0
    }
  },
  { immediate: true },
)

// 进入大地图自定义时确保 last_rows/last_cols 默认为 2×2
watch(
  isLargeMapCustom,
  async (custom) => {
    if (!custom) return
    const cur = settingsStore.settings
    if (!cur) return
    if (!cur.last_rows || cur.last_rows < 1) await persistSetting('last_rows', 2)
    if (!cur.last_cols || cur.last_cols < 1) await persistSetting('last_cols', 2)
    // 重置本地编辑值，使其回落到 last_rows/last_cols
    localGridRows.value = 0
    localGridCols.value = 0
  },
  { immediate: true },
)

async function onScrollModeSelect(event: Event) {
  const target = event.target as HTMLSelectElement
  const name = target.value
  configStore.currentScrollModeName = name
  await persistSetting('last_scroll_mode', name)
}

async function onGridRowsInput(event: Event) {
  if (!isGridEditable.value) return
  const num = clampGridNum((event.target as HTMLInputElement).value)
  if (num === null) return
  localGridRows.value = num
  await persistSetting('last_rows', num)
  await saveGridEdit()
}

async function onGridColsInput(event: Event) {
  if (!isGridEditable.value) return
  const num = clampGridNum((event.target as HTMLInputElement).value)
  if (num === null) return
  localGridCols.value = num
  await persistSetting('last_cols', num)
  await saveGridEdit()
}

/** 网格输入解析：1-100 之间整数，否则返回 null */
function clampGridNum(raw: string): number | null {
  const num = parseInt(raw, 10)
  if (Number.isNaN(num)) return null
  return Math.max(1, Math.min(100, num))
}

/** 保存网格编辑到数据库（仅更新 grid_rows/grid_cols，保留其他字段） */
async function saveGridEdit() {
  const base = configStore.currentRegion
  if (!base) return
  const targetRows = localGridRows.value
  const targetCols = localGridCols.value
  if (targetRows < 1 || targetCols < 1) return
  try {
    const updated: RegionConfig = {
      ...base,
      grid_rows: targetRows,
      grid_cols: targetCols,
      updated_at: new Date().toISOString(),
    }
    await configStore.upsertRegion(updated)
  } catch (e) {
    console.error('[ConfigPanel] 保存网格编辑失败:', e)
  }
}

async function onFormatChange(value: 'JPG' | 'PNG') {
  await persistSetting('output_format', value)
}

async function onJpgQualityChange(value: number) {
  if (Number.isNaN(value)) return
  await persistSetting('jpg_quality', Math.max(1, Math.min(100, value)))
}
</script>

<style scoped>
.config-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 14px;
  height: 100%;
  overflow-y: auto;
  color: var(--text-primary);
}

.loading {
  padding: 24px;
  text-align: center;
  color: var(--text-muted);
  font-size: 12px;
}

.layer {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.field-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 11px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 6px;
}

.field-select,
.field-input {
  padding: 4px 8px;
  background: var(--input-bg);
  color: var(--text-primary);
  border: 1px solid var(--input-border);
  border-radius: 4px;
  font-size: 12px;
  font-family: inherit;
  width: 100%;
  box-sizing: border-box;
  transition: border-color 0.15s ease;
}

.field-select:focus,
.field-input:focus {
  border-color: var(--accent);
}

.field-select:disabled,
.field-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.field-input[readonly] {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
  cursor: default;
}

/* 区域主类/子名并排 */
.region-pair {
  display: flex;
  gap: 6px;
}

.region-pair .field-select {
  flex: 1;
  min-width: 0;
}

.number-pair {
  display: flex;
  align-items: center;
  gap: 6px;
}

.number-pair .field-input {
  flex: 1;
  min-width: 0;
}

.pair-sep {
  color: var(--text-muted);
  font-size: 11px;
  user-select: none;
}

/* JPG 质量拖拽条 */
.quality-slider {
  width: 100%;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: var(--input-border);
  border-radius: 2px;
  outline: none;
  cursor: pointer;
}

.quality-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--accent);
  cursor: pointer;
  border: 1px solid var(--bg-primary);
}

.quality-slider::-moz-range-thumb {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--accent);
  cursor: pointer;
  border: 1px solid var(--bg-primary);
}
</style>
