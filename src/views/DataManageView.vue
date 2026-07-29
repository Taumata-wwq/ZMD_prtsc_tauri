<template>
  <div class="data-view">
    <header class="toolbar">
      <h2 class="title">{{ t('data.title') }}</h2>
      <div class="toolbar-actions">
        <span v-if="loading" class="loading-hint">{{ t('common.loading') }}</span>
        <span v-else-if="error" class="error-hint" :title="error">{{ error }}</span>
        <button class="btn btn-primary" type="button" :disabled="loading" @click="onAdd">{{ t('data.add') }}</button>
      </div>
    </header>

    <div class="data-scroll">
      <!-- 树状结构：按 category（地区）分组 -->
      <section v-for="cat in treeData" :key="cat.name" class="tree-group">
        <div class="tree-header" @click="toggleCategory(cat.name)">
          <svg class="tree-arrow" :class="{ expanded: expandedCategories.has(cat.name) }" width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
            <path d="M9 18l6-6-6-6v12z" />
          </svg>
          <span class="tree-name">{{ cat.name }}</span>
          <span class="tree-count">{{ cat.items.length }}</span>
        </div>
        <div v-show="expandedCategories.has(cat.name)" class="tree-children">
          <div
              v-for="item in cat.items"
              :key="item.name"
              class="tree-item"
              @click="onItemClick(item)"
              @focusout="onItemFocusout($event)"
            >
              <!-- 显示模式：单行显示 name + target + 操作按钮 -->
              <template v-if="editingName !== item.name">
                <div class="item-row">
                  <span class="item-name" :title="item.name">{{ stripPrefix(item.name) }}</span>
                  <span class="item-target">{{ item.targetW }} × {{ item.targetH }}</span>
                  <div class="item-actions">
                    <button class="btn btn-sm btn-danger" @click.stop="onDelete(item)">{{ t('data.delete') }}</button>
                  </div>
                </div>
              </template>

              <!-- 编辑模式：第一行输入 + 第二行常驻测算数据 -->
              <template v-else>
                <div class="item-row editing-row">
                  <input
                    ref="editNameInputEl"
                    v-model="editing.name"
                    class="item-input name-input"
                    :placeholder="t('data.name')"
                  />
                  <div class="target-group">
                    <input v-model.number="editTargetW" type="number" class="item-input target-input" min="908" max="16000" placeholder="W" />
                    <span class="target-sep">×</span>
                    <input v-model.number="editTargetH" type="number" class="item-input target-input" min="528" max="16000" placeholder="H" />
                  </div>
                </div>
              <!-- 第二行：常驻显示 9 个滚动次数（0-8）的测算数据 -->
              <div class="item-derived">
                <template v-if="allCountsResult">
                  <div
                    v-for="(count, idx) in allCountsResult.counts"
                    :key="idx"
                    class="mode-row"
                  >
                    <span class="mode-label">{{ idx }}次</span>
                    <span>drag: {{ count.drag_x }}, {{ count.drag_y }}</span>
                    <span>grid: {{ count.actual_rows }}×{{ count.actual_cols }}</span>
                    <span>overlap: {{ (count.overlap_x * 100).toFixed(1) }}%, {{ (count.overlap_y * 100).toFixed(1) }}%</span>
                  </div>
                </template>
                <span v-else class="derived-hint">{{ t('data.inputTarget') }}</span>
              </div>
            </template>
          </div>
        </div>
      </section>

      <!-- 新增区域 -->
      <section v-if="isAdding" class="tree-group">
        <div class="tree-header">
          <span class="tree-name">{{ t('data.add') }}</span>
        </div>
        <div class="tree-children">
          <div class="tree-item">
            <div class="item-row editing-row">
              <select v-model="editing.category" class="item-select">
                <option v-for="cat in availableCategories" :key="cat" :value="cat">{{ cat }}</option>
                <option value="__custom__">{{ t('data.customCategory') }}</option>
              </select>
              <input
                v-if="editing.category === '__custom__'"
                v-model="customCategory"
                class="item-input cat-input"
                :placeholder="t('data.category')"
              />
              <input v-model="editing.name" class="item-input name-input" :placeholder="t('data.name')" />
              <div class="target-group">
                <input v-model.number="editTargetW" type="number" class="item-input target-input" min="908" max="16000" placeholder="W" />
                <span class="target-sep">×</span>
                <input v-model.number="editTargetH" type="number" class="item-input target-input" min="528" max="16000" placeholder="H" />
              </div>
              <div class="item-actions">
                <button class="btn btn-sm btn-primary" @click="onSaveNew">{{ t('data.save') }}</button>
                <button class="btn btn-sm" @click="cancelAdd">{{ t('data.cancel') }}</button>
              </div>
            </div>
            <div class="item-derived">
              <template v-if="allCountsResult">
                <div
                  v-for="(count, idx) in allCountsResult.counts"
                  :key="idx"
                  class="mode-row"
                >
                  <span class="mode-label">{{ idx }}次</span>
                  <span>drag: {{ count.drag_x }}, {{ count.drag_y }}</span>
                  <span>grid: {{ count.actual_rows }}×{{ count.actual_cols }}</span>
                  <span>overlap: {{ (count.overlap_x * 100).toFixed(1) }}%, {{ (count.overlap_y * 100).toFixed(1) }}%</span>
                </div>
              </template>
              <span v-else class="derived-hint">{{ t('data.inputTarget') }}</span>
            </div>
          </div>
        </div>
      </section>

      <div v-if="!loading && regions.length === 0 && !isAdding" class="empty-hint">{{ t('data.empty') }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, onDeactivated, nextTick } from 'vue'
import { api, deriveAllCounts } from '@/api'
import { useConfigStore, sortRegions } from '@/stores/config.store'
import { useSettingsStore } from '@/stores/settings.store'
import { useI18n } from '@/composables/useI18n'
import type { RegionConfig, AllCountsResult } from '@/types'

const configStore = useConfigStore()
const settingsStore = useSettingsStore()
const { t } = useI18n()

const regions = ref<RegionConfig[]>([])
const loading = ref(false)
const error = ref<string | null>(null)

/** 编辑状态：编辑现有区域时存 name，新增时 isAdding=true */
const editingName = ref<string | null>(null)
const isAdding = ref(false)
const editing = ref<RegionConfig>(createEmpty())
const customCategory = ref('')
/** 编辑模式下 name 输入框的 DOM 引用（用于自动聚焦） */
const editNameInputEl = ref<HTMLInputElement | null>(null)

const editTargetW = ref<number>(0)
const editTargetH = ref<number>(0)
const allCountsResult = ref<AllCountsResult | null>(null)
/** 跳过自动保存标志：startEdit 设置 target 时置 true，避免首次赋值触发 watch 就自动保存 */
let skipAutoSave = false

/** 展开的分类集合 */
const expandedCategories = ref<Set<string>>(new Set())

/** 固定的客户端基准尺寸（drag 值与游戏分辨率无关，使用 Python 原版基准） */
const CLIENT_W = 1920
const CLIENT_H = 1080

/** 自动保存定时器 */
let autoSaveTimer: ReturnType<typeof setTimeout> | null = null

/** 按 name 分组的区域项（取 0次 作为代表） */
interface RegionGroup {
  name: string
  category: string
  records: Map<string, RegionConfig>
  representative: RegionConfig
  targetW: number
  targetH: number
}

/** 已有的分类列表（排除大地图和自定义） */
const availableCategories = computed<string[]>(() => {
  const set = new Set<string>()
  regions.value.forEach((r) => {
    if (r.category && r.category !== '大地图' && r.category !== '自定义') set.add(r.category)
  })
  return Array.from(set)
})

/** 树状数据：按 category 分组 → 按 name 分组（排除大地图和自定义） */
const treeData = computed<{ name: string; items: RegionGroup[] }[]>(() => {
  // 先按 name 分组
  const nameMap = new Map<string, RegionGroup>()
  regions.value.forEach((r) => {
    if (r.category === '大地图' || r.category === '自定义') return
    if (!nameMap.has(r.name)) {
      nameMap.set(r.name, {
        name: r.name,
        category: r.category,
        records: new Map(),
        representative: r,
        targetW: 0,
        targetH: 0,
      })
    }
    const group = nameMap.get(r.name)!
    group.records.set(r.scroll_mode, r)
    // 0次 作为代表
    if (r.scroll_mode === '0次') {
      group.representative = r
    }
  })
  // 计算 target 并按 category 分组
  const catMap = new Map<string, RegionGroup[]>()
  nameMap.forEach((group) => {
    const target = calcTarget(group.representative)
    group.targetW = target.w
    group.targetH = target.h
    const cat = group.category || '未分类'
    if (!catMap.has(cat)) catMap.set(cat, [])
    catMap.get(cat)!.push(group)
  })
  return Array.from(catMap.entries()).map(([name, items]) => ({ name, items }))
})

function toggleCategory(name: string) {
  if (expandedCategories.value.has(name)) {
    expandedCategories.value.delete(name)
  } else {
    expandedCategories.value.add(name)
  }
}

/**
 * 前端计算 target_w/target_h（与 Rust stitcher.rs 截断公式一致）
 * 使用统一的 clientSize，保证与 deriveAllCounts 一致
 */
function calcTarget(region: RegionConfig): { w: number; h: number } {
  // 优先使用存储的 target_w/target_h（用户输入的源真值）
  if (region.target_w > 0 && region.target_h > 0) {
    return { w: region.target_w, h: region.target_h }
  }
  // 旧记录未迁移，从 grid/overlap 反算
  const imgW = Math.round(CLIENT_W * region.capture_region_x)
  const imgH = Math.round(CLIENT_H * region.capture_region_y)
  const ovlpPxX = Math.trunc(imgW * region.overlap_x)
  const ovlpPxY = Math.trunc(imgH * region.overlap_y)
  const stepX = imgW - ovlpPxX
  const stepY = imgH - ovlpPxY
  const targetW = stepX * region.grid_cols + ovlpPxX
  const targetH = stepY * region.grid_rows + ovlpPxY
  return { w: targetW, h: targetH }
}

function createEmpty(): RegionConfig {
  const now = new Date().toISOString()
  return {
    name: '',
    category: '武陵',
    aspect_ratio: '16:9',
    scroll_mode: '0次',
    grid_rows: 2,
    grid_cols: 2,
    overlap_x: 0.001,
    overlap_y: 0.001,
    drag_x: 905,
    drag_y: 525,
    capture_region_x: 0.626,
    capture_region_y: 0.648,
    capture_offset_y: 0,
    template_ref: null,
    target_w: 0,
    target_h: 0,
    created_at: now,
    updated_at: now,
  }
}

/** 去掉地名前缀：name 中如果包含"-"，取"-"后面的部分；否则显示完整 name */
function stripPrefix(name: string): string {
  const idx = name.indexOf('-')
  return idx >= 0 ? name.slice(idx + 1) : name
}

/** 提取地名前缀：name 中如果包含"-"，取"-"前面的部分；否则返回空字符串 */
function getCategoryPrefix(name: string): string {
  const idx = name.indexOf('-')
  return idx >= 0 ? name.slice(0, idx) : ''
}

/** 监听 target 变化，实时调用 deriveAllCounts 并自动保存 */
watch(
  [editTargetW, editTargetH],
  async ([w, h]) => {
    if (w <= 0 || h <= 0) {
      allCountsResult.value = null
      return
    }
    // 首次赋值（startEdit 设置 target）跳过自动保存，仅推导显示
    const wasSkipAutoSave = skipAutoSave
    skipAutoSave = false
    try {
      const overlapMin = settingsStore.settings?.overlap_min ?? 0.0
      const overlapMax = settingsStore.settings?.overlap_max ?? 0.5
      const result = await deriveAllCounts(CLIENT_W, CLIENT_H, w, h, overlapMin, overlapMax)
      allCountsResult.value = result
      // 仅在用户真正修改值时自动保存（跳过 startEdit 的首次赋值）
      if (!wasSkipAutoSave && !isAdding.value && editingName.value) {
        scheduleAutoSave(result)
      }
    } catch (e) {
      console.error('[DataManageView] deriveAllCounts 失败:', e)
      allCountsResult.value = null
    }
  },
)

/** 手动触发一次推导（用于 startEdit 后确保 allCountsResult 不为 null） */
async function rederive() {
  if (editTargetW.value > 0 && editTargetH.value > 0) {
    const overlapMin = settingsStore.settings?.overlap_min ?? 0.0
    const overlapMax = settingsStore.settings?.overlap_max ?? 0.5
    try {
      const result = await deriveAllCounts(CLIENT_W, CLIENT_H, editTargetW.value, editTargetH.value, overlapMin, overlapMax)
      allCountsResult.value = result
    } catch (e) {
      console.error('[DataManageView] rederive 失败:', e)
      allCountsResult.value = null
    }
  }
}

function scheduleAutoSave(allCounts: AllCountsResult | null) {
  if (autoSaveTimer) clearTimeout(autoSaveTimer)
  autoSaveTimer = setTimeout(async () => {
    await saveExistingRegion(allCounts)
  }, 800)
}

/** 保存现有区域：仅更新 0次 记录（用 target 反推 grid/overlap/drag） */
async function saveExistingRegion(
  allCounts: AllCountsResult | null,
) {
  if (!editingName.value || !allCounts) return
  // 拼接完整 name：原始前缀 + 当前输入值（去掉前缀的部分）
  const prefix = getCategoryPrefix(editingName.value)
  const inputName = editing.value.name
  const name = inputName
    ? (prefix ? `${prefix}-${inputName}` : inputName)
    : editingName.value
  // 找到原始 group 的 category 和 0次记录
  let category = editing.value.category
  let originalRegion0: RegionConfig | undefined
  for (const cat of treeData.value) {
    for (const item of cat.items) {
      if (item.name === editingName.value) {
        category = item.category
        originalRegion0 = item.records.get('0次')
        break
      }
    }
  }
  if (!originalRegion0) return
  // 用 0次的推导结果反推 grid/overlap/drag
  const derived0 = allCounts.counts[0]
  const now = new Date().toISOString()
  try {
    const cfg: RegionConfig = {
      ...originalRegion0,
      name,
      category,
      scroll_mode: '0次',
      grid_rows: derived0.actual_rows,
      grid_cols: derived0.actual_cols,
      overlap_x: derived0.overlap_x,
      overlap_y: derived0.overlap_y,
      drag_x: derived0.drag_x,
      drag_y: derived0.drag_y,
      target_w: editTargetW.value,  // 保存用户输入的 target
      target_h: editTargetH.value,  // 保存用户输入的 target
      updated_at: now,
    }
    await api.upsertRegion(cfg)
    await refresh()
    // 同步 configStore：先 reload 列表，再根据当前选中区域重新推导
    await configStore.load()
    if (name !== editingName.value && configStore.currentRegionName === editingName.value) {
      configStore.currentRegionName = name
    }
    await configStore.refreshCurrentRegion()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
}

async function refresh() {
  loading.value = true
  error.value = null
  try {
    regions.value = sortRegions(await api.listRegions())
    expandedCategories.value = new Set(treeData.value.map((c) => c.name))
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

/** 点击行进入编辑模式 */
function onItemClick(item: RegionGroup) {
  // 新增模式下不处理，避免与新增表单冲突
  if (isAdding.value) return
  // 已在编辑此项，不重复触发
  if (editingName.value === item.name) return
  // 切换编辑项前，先保存当前项的 pending 修改，避免丢失
  flushPendingSave()
  startEdit(item)
}

/** 失去焦点后收起编辑模式（回到显示模式） */
function onItemFocusout(e: FocusEvent) {
  if (!editingName.value) return
  const next = e.relatedTarget as Node | null
  const container = e.currentTarget as HTMLElement
  // 焦点仍在容器内，不收起
  if (next && container.contains(next)) return
  cancelEdit()
}

/** 进入编辑模式 */
async function startEdit(item: RegionGroup) {
  editingName.value = item.name
  isAdding.value = false
  editing.value = { ...item.representative }
  // 编辑框只显示去掉前缀的部分（保存时会重新拼接完整名称）
  editing.value.name = stripPrefix(item.name)
  // 不在此处清空 allCountsResult，避免 rederive 完成前 UI 显示"请输入目标尺寸"
  // 标记跳过首次赋值的自动保存，避免点击编辑但未修改时也触发 upsert
  skipAutoSave = true
  editTargetW.value = item.targetW
  editTargetH.value = item.targetH
  // 先聚焦到 name 输入框，让用户立即看到焦点反馈
  await nextTick()
  editNameInputEl.value?.focus()
  editNameInputEl.value?.select()
  // 等待推导完成，避免 null 中间状态导致 UI 显示"请输入目标尺寸"
  // 对于 target 相同的区域切换（如景玉谷→首敦），watch 不触发，必须由 rederive 更新
  await rederive()
}

function cancelEdit() {
  // 先 flush pending save，再清空状态
  // SAFETY: saveExistingRegion 会同步执行到 await api.upsertRegion(cfg)，
  // cfg 在 await 之前已构造完成，之后清空 editingName 不会影响保存
  flushPendingSave()
  editingName.value = null
  allCountsResult.value = null
}

/** 立即执行可能 pending 的自动保存（fire-and-forget） */
function flushPendingSave() {
  if (!autoSaveTimer) return
  clearTimeout(autoSaveTimer)
  autoSaveTimer = null
  const pendingAllCounts = allCountsResult.value
  if (pendingAllCounts && editingName.value) {
    void saveExistingRegion(pendingAllCounts)
  }
}

/** 新增区域：仅插入 0次 记录（用 target 反推 grid/overlap/drag） */
async function onSaveNew() {
  if (!editing.value.name || editTargetW.value <= 0 || editTargetH.value <= 0) {
    error.value = '请填写完整信息'
    return
  }
  const overlapMin = settingsStore.settings?.overlap_min ?? 0.0
  const overlapMax = settingsStore.settings?.overlap_max ?? 0.5
  try {
    const result = await deriveAllCounts(CLIENT_W, CLIENT_H, editTargetW.value, editTargetH.value, overlapMin, overlapMax)
    if (!result) {
      error.value = '推导失败'
      return
    }
    const category = editing.value.category === '__custom__' && customCategory.value
      ? customCategory.value
      : editing.value.category
    const name = editing.value.name
    const now = new Date().toISOString()
    // 用 0次的推导结果作为 0次记录的 grid/overlap/drag
    const derived0 = result.counts[0]
    const cfg: RegionConfig = {
      ...createEmpty(),
      name,
      category,
      scroll_mode: '0次',
      grid_rows: derived0.actual_rows,
      grid_cols: derived0.actual_cols,
      overlap_x: derived0.overlap_x,
      overlap_y: derived0.overlap_y,
      drag_x: derived0.drag_x,
      drag_y: derived0.drag_y,
      target_w: editTargetW.value,
      target_h: editTargetH.value,
      created_at: now,
      updated_at: now,
    }
    await api.upsertRegion(cfg)
    await refresh()
    await configStore.load()
    await configStore.refreshCurrentRegion()
    cancelAdd()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
}

function onAdd() {
  isAdding.value = true
  editingName.value = null
  editing.value = createEmpty()
  allCountsResult.value = null
  skipAutoSave = true
  editTargetW.value = 0
  editTargetH.value = 0
  customCategory.value = ''
}

function cancelAdd() {
  isAdding.value = false
  editing.value = createEmpty()
  allCountsResult.value = null
  editTargetW.value = 0
  editTargetH.value = 0
  customCategory.value = ''
}

async function onDelete(group: RegionGroup) {
  if (!confirm(t('data.confirmDelete'))) return
  try {
    // 新架构：普通区域只有 0次记录，直接删除
    // 兼容旧数据库：遍历所有记录删除
    for (const rec of group.records.values()) {
      await api.deleteRegion(rec.name, rec.aspect_ratio, rec.scroll_mode)
    }
    await refresh()
    await configStore.load()
    await configStore.refreshCurrentRegion()
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
}

onMounted(() => {
  refresh()
})

onDeactivated(() => {
  flushPendingSave()
})

onUnmounted(() => {
  // 组件卸载前 flush pending save，避免修改丢失
  flushPendingSave()
})
</script>

<style scoped>
.data-view {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--bg-primary);
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.title {
  font-size: 14px;
  font-weight: 600;
  margin: 0;
  color: var(--text-primary);
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.loading-hint,
.error-hint {
  font-size: 12px;
  color: var(--text-muted);
}

.error-hint {
  color: #e81123;
}

.data-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
  max-width: 700px;
  width: 100%;
  margin: 0 auto;
}

/* 树状结构 */
.tree-group {
  margin-bottom: 4px;
}

.tree-header {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 8px;
  cursor: pointer;
  user-select: none;
  border-radius: 3px;
  transition: background 0.12s ease;
}

.tree-header:hover {
  background: var(--bg-tertiary);
}

.tree-arrow {
  flex-shrink: 0;
  transition: transform 0.15s ease;
  fill: var(--text-muted);
}

.tree-arrow.expanded {
  transform: rotate(90deg);
}

.tree-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  flex: 1;
}

.tree-count {
  font-size: 10px;
  color: var(--text-muted);
  background: var(--bg-tertiary);
  padding: 0 6px;
  border-radius: 8px;
  line-height: 14px;
}

.tree-children {
  margin-left: 12px;
}

.tree-item {
  border-bottom: 1px solid var(--border);
}

.tree-item:last-child {
  border-bottom: none;
}

/* 单行数据项 */
.item-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  min-height: 32px;
}

.editing-row {
  flex-wrap: wrap;
  gap: 4px;
  padding: 8px;
  background: var(--bg-tertiary);
}

.item-name {
  font-size: 12px;
  color: var(--text-primary);
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-target {
  font-size: 11px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
  flex-shrink: 0;
  width: 120px;
  text-align: right;
}

.item-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

/* 编辑模式输入控件 */
.item-select {
  height: 24px;
  padding: 0 4px;
  border: 1px solid var(--input-border);
  border-radius: 3px;
  background: var(--input-bg);
  color: var(--text-primary);
  font-size: 11px;
  font-family: inherit;
  cursor: pointer;
  outline: none;
  flex-shrink: 0;
}

.item-input {
  height: 24px;
  padding: 0 4px;
  border: 1px solid var(--input-border);
  border-radius: 3px;
  background: var(--input-bg);
  color: var(--text-primary);
  font-size: 11px;
  font-family: inherit;
  outline: none;
  box-sizing: border-box;
}

.item-input:focus,
.item-select:focus {
  border-color: var(--accent);
}

.cat-input {
  width: 80px;
  flex-shrink: 0;
}

.name-input {
  flex: 1;
  min-width: 80px;
}

.target-group {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

.target-input {
  width: 60px;
  text-align: center;
}

.target-sep {
  font-size: 11px;
  color: var(--text-muted);
}

/* 第二行：常驻测算数据 */
.item-derived {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 8px 6px;
  font-size: 10px;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
  background: var(--bg-tertiary);
}

.mode-row {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  align-items: center;
}

.mode-label {
  color: var(--text-secondary);
  font-weight: 600;
  min-width: 56px;
}

.derived-hint {
  font-style: italic;
}

/* 按钮 */
.btn {
  height: 24px;
  padding: 0 8px;
  border: 1px solid var(--border);
  border-radius: 3px;
  background: var(--bg-tertiary);
  color: var(--text-primary);
  font-size: 11px;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
  white-space: nowrap;
}

.btn:hover:not(:disabled) {
  background: var(--btn-hover-bg);
  border-color: var(--accent);
}

.btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn.btn-sm {
  height: 20px;
  padding: 0 6px;
  font-size: 10px;
}

.btn.btn-primary {
  background: var(--accent);
  color: #ffffff;
  border-color: var(--accent);
}

.btn.btn-primary:hover:not(:disabled) {
  background: var(--accent-hover);
  border-color: var(--accent-hover);
}

.btn.btn-danger {
  background: #e81123;
  color: #ffffff;
  border-color: #e81123;
}

.btn.btn-danger:hover:not(:disabled) {
  background: #c50f1f;
  border-color: #c50f1f;
}

.empty-hint {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-muted);
  font-size: 13px;
}
</style>