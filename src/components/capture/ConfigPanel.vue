<template>
  <div class="config-panel">
    <div v-if="!settingsStore.settings" class="loading">{{ t('common.loading') }}</div>

    <template v-else>
      <section class="layer basic-layer">
        <!-- 区域选择分两部分（主类/子名） -->
        <div class="field-group">
          <label class="field-label">{{ t('capture.region') }}</label>
          <div class="region-pair">
            <!-- 主类选择（武陵/谷地/大地图/自定义） -->
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
            <!-- 子名选择（根据主类动态生成；大地图/自定义时禁用） -->
            <select
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

        <!-- 滚动模式（大地图/自定义模式禁用，固定为0次） -->
        <div class="field-group">
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

        <!-- 网格大小 -->
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

        <!-- 输出格式与质量 -->
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
import type { RegionConfig } from '@/types'

const configStore = useConfigStore()
const settingsStore = useSettingsStore()
const captureStore = useCaptureStore()
const { t } = useI18n()

/** 所有区域按 category 分组 */
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

/** 当前主类（从 currentRegionName 解析） */
const currentCategory = computed<string>(() => {
  const name = configStore.currentRegionName
  if (!name) return categories.value[0] ?? ''
  // 大地图/自定义：name 即 category
  if (name === '大地图' || name === '自定义') return name
  // 其他：<category>-<sub>
  const idx = name.indexOf('-')
  return idx > 0 ? name.substring(0, idx) : name
})

/** 当前主类下的子名列表（去除 category 前缀） */
const subNames = computed<string[]>(() => {
  const cat = currentCategory.value
  if (!cat) return []
  const names = regionsByCategory.value[cat] ?? []
  // 对于 大地图/自定义，子名为空（整个 name 就是 category）
  if (cat === '大地图' || cat === '自定义') return []
  // 提取 "-" 后的子名
  return names
    .map((n) => {
      const idx = n.indexOf('-')
      return idx > 0 ? n.substring(idx + 1) : n
    })
    .filter((v, i, arr) => arr.indexOf(v) === i) // 去重
})

/** 是否有子名可选（大地图/自定义无） */
const hasSubNames = computed<boolean>(() => subNames.value.length > 0)

/** 当前子名（从 currentRegionName 解析） */
const currentSubName = computed<string>(() => {
  const name = configStore.currentRegionName
  if (!name) return ''
  if (name === '大地图' || name === '自定义') return ''
  const idx = name.indexOf('-')
  return idx > 0 ? name.substring(idx + 1) : ''
})

/** 拼接完整 region name */
function buildRegionName(category: string, subName: string): string {
  if (category === '大地图' || category === '自定义') return category
  if (!subName) return category
  return `${category}-${subName}`
}

/** 主类切换：选第一个子名（或直接使用 category） */
async function onCategoryChange(event: Event) {
  const target = event.target as HTMLSelectElement
  const category = target.value
  const subs = (regionsByCategory.value[category] ?? [])
    .map((n) => {
      if (category === '大地图' || category === '自定义') return ''
      const idx = n.indexOf('-')
      return idx > 0 ? n.substring(idx + 1) : n
    })
    .filter((v, i, arr) => v && arr.indexOf(v) === i)
  const sub = subs[0] ?? ''
  const fullName = buildRegionName(category, sub)
  configStore.currentRegionName = fullName
  try {
    await settingsStore.update('last_region', fullName)
  } catch (e) {
    console.error('[ConfigPanel] 更新 last_region 失败:', e)
  }
  // 切换到大地图/自定义时固定滚动模式为0次
  if (category === '大地图' || category === '自定义') {
    configStore.currentScrollModeName = '0次'
    try {
      await settingsStore.update('last_scroll_mode', '0次')
    } catch (e) {
      console.error('[ConfigPanel] 更新 last_scroll_mode 失败:', e)
    }
  }
}

/** 子名切换 */
async function onSubNameChange(event: Event) {
  const target = event.target as HTMLSelectElement
  const sub = target.value
  const fullName = buildRegionName(currentCategory.value, sub)
  configStore.currentRegionName = fullName
  try {
    await settingsStore.update('last_region', fullName)
  } catch (e) {
    console.error('[ConfigPanel] 更新 last_region 失败:', e)
  }
}

// ---------------------------------------------------------------------------
// 大地图/自定义模式判断（固定滚动次数，不可切换）
// ---------------------------------------------------------------------------
const isStaticMode = computed<boolean>(() => {
  const cat = currentCategory.value
  return cat === '自定义' || cat === '大地图'
})

/** 网格是否可编辑：自定义 或 大地图 */
const isGridEditable = computed<boolean>(() => {
  const cat = currentCategory.value
  return cat === '自定义' || cat === '大地图'
})

// ---------------------------------------------------------------------------
// 网格大小（仅自定义/大地图可编辑）
// ---------------------------------------------------------------------------
const localGridRows = ref<number>(0)
const localGridCols = ref<number>(0)

const gridRows = computed<number>(() => {
  if (isGridEditable.value) {
    return localGridRows.value || (settingsStore.settings?.last_rows ?? 0)
  }
  return configStore.currentRegion?.grid_rows ?? 0
})

const gridCols = computed<number>(() => {
  if (isGridEditable.value) {
    return localGridCols.value || (settingsStore.settings?.last_cols ?? 0)
  }
  return configStore.currentRegion?.grid_cols ?? 0
})

// 监听 currentRegion 变化，重置本地编辑值
watch(
  () => configStore.currentRegion,
  (r) => {
    if (r && isGridEditable.value) {
      localGridRows.value = r.grid_rows
      localGridCols.value = r.grid_cols
    } else {
      localGridRows.value = 0
      localGridCols.value = 0
    }
  },
  { immediate: true },
)

// ---------------------------------------------------------------------------
// 滚动模式切换
// ---------------------------------------------------------------------------
async function onScrollModeSelect(event: Event) {
  const target = event.target as HTMLSelectElement
  const name = target.value
  configStore.currentScrollModeName = name
  try {
    await settingsStore.update('last_scroll_mode', name)
  } catch (e) {
    console.error('[ConfigPanel] 更新 last_scroll_mode 失败:', e)
  }
}

// ---------------------------------------------------------------------------
// 网格大小编辑（仅自定义/大地图）
// ---------------------------------------------------------------------------
async function onGridRowsInput(event: Event) {
  if (!isGridEditable.value) return
  const target = event.target as HTMLInputElement
  let num = parseInt(target.value, 10)
  if (Number.isNaN(num)) return
  if (num < 1) num = 1
  if (num > 100) num = 100
  localGridRows.value = num
  try {
    await settingsStore.update('last_rows', num)
  } catch (e) {
    console.error('[ConfigPanel] 更新 last_rows 失败:', e)
  }
  await saveGridEdit()
}

async function onGridColsInput(event: Event) {
  if (!isGridEditable.value) return
  const target = event.target as HTMLInputElement
  let num = parseInt(target.value, 10)
  if (Number.isNaN(num)) return
  if (num < 1) num = 1
  if (num > 100) num = 100
  localGridCols.value = num
  try {
    await settingsStore.update('last_cols', num)
  } catch (e) {
    console.error('[ConfigPanel] 更新 last_cols 失败:', e)
  }
  await saveGridEdit()
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

// ---------------------------------------------------------------------------
// 输出格式与 JPG 质量控制
// ---------------------------------------------------------------------------
async function onFormatChange(value: 'JPG' | 'PNG') {
  try {
    await settingsStore.update('output_format', value)
  } catch (e) {
    console.error('[ConfigPanel] 更新输出格式失败:', e)
  }
}

async function onJpgQualityChange(value: number) {
  if (Number.isNaN(value)) return
  const clamped = Math.max(1, Math.min(100, value))
  try {
    await settingsStore.update('jpg_quality', clamped)
  } catch (e) {
    console.error('[ConfigPanel] 更新 JPG 质量失败:', e)
  }
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

.quality-slider:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
