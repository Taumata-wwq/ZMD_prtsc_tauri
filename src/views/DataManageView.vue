<template>
  <div class="data-view" @click="closeCtxMenu">
    <header class="data-titlebar">
      <h2 class="data-title">{{ t('data.title') }}</h2>
    </header>

    <!-- 主体：左侧树 + 右侧编辑区 -->
    <div class="data-body">
      <aside
        class="sidebar"
        :style="{ width: leftWidth + 'px', flexBasis: leftWidth + 'px' }"
      >
        <div class="list-scroll" @contextmenu.prevent="onRootContextmenu">
          <div v-if="loading" class="empty-hint">{{ t('common.loading') }}</div>
          <div v-else-if="error" class="empty-hint error-hint">{{ error }}</div>

          <!-- 基建类别（四号谷地 / 武陵） -->
          <section
            v-for="cat in baseTree"
            :key="cat.name"
            class="tree-group"
          >
            <div
              class="tree-header lv1"
              @click="toggleExpand(cat.name)"
              @contextmenu.prevent.stop="onCategoryContextmenu($event, cat.name)"
            >
              <svg
                class="tree-arrow"
                :class="{ expanded: expanded.has(cat.name) }"
                width="8" height="8" viewBox="0 0 24 24" fill="currentColor"
              >
                <path d="M9 18l6-6-6-6v12z" />
              </svg>
              <span class="tree-name bold">{{ cat.name }}</span>
              <span class="tree-count">{{ cat.items.length }}</span>
            </div>
            <ul v-show="expanded.has(cat.name)" class="tree-children">
              <li
                v-for="item in cat.items"
                :key="item.name"
                class="tree-item lv2"
                :class="{ active: selectedKey === makeBaseKey(item.name) }"
                @click="onSelectBaseRegion(item)"
                @contextmenu.prevent.stop="onBaseRegionContextmenu($event, item.name)"
              >
                <span class="tree-name" :title="stripCategoryPrefix(item.name)">{{ stripCategoryPrefix(item.name) }}</span>
                <span class="tree-meta">{{ item.targetW }}×{{ item.targetH }}</span>
              </li>
            </ul>
          </section>

          <!-- 大地图（顶级） -->
          <section class="tree-group">
            <div
              class="tree-header lv1"
              @click="toggleExpand('大地图')"
              @contextmenu.prevent.stop="onLargeMapRootContextmenu($event)"
            >
              <svg
                class="tree-arrow"
                :class="{ expanded: expanded.has('大地图') }"
                width="8" height="8" viewBox="0 0 24 24" fill="currentColor"
              >
                <path d="M9 18l6-6-6-6v12z" />
              </svg>
              <span class="tree-name bold">{{ t('data.tabLargeMap') }}</span>
              <span class="tree-count">{{ totalLargeMapCount }}</span>
            </div>
            <ul v-show="expanded.has('大地图')" class="tree-children">
              <li
                v-for="sm in largeMapSubMaps"
                :key="sm.name"
                class="tree-item lv2"
                :class="{ active: selectedKey === makeLargeMapSubKey(sm.name) }"
                @click="onSelectLargeMapSub(sm.name)"
                @contextmenu.prevent.stop="onLargeMapSubContextmenu($event, sm.name)"
              >
                <span class="tree-name">{{ sm.name }}</span>
                <span class="tree-count">{{ sm.count }}</span>
              </li>
            </ul>
          </section>

          <div
            v-if="baseTree.length === 0 && largeMapSubMaps.length === 0"
            class="empty-hint"
          >{{ t('data.empty') }}</div>
        </div>
      </aside>

      <div class="splitter" @mousedown="onSplitterMouseDown">
        <div class="splitter-handle" />
      </div>

      <!-- 右侧编辑面板（最大宽度 650px，居中） -->
      <section class="edit-pane">
        <div class="edit-wrap">
          <template v-if="currentPanel === 'base'">
            <DataRegionEditPanel
              v-model:targetW="editTargetW"
              v-model:targetH="editTargetH"
              :region="selectedBaseRegion"
              :all-counts-result="allCountsResult"
            />
          </template>

          <!-- 大地图子地图编辑（仅显示子地区列表，无名称编辑） -->
          <template v-else-if="currentPanel === 'largeMapSub'">
            <DataLargeMapEditPanel
              :sub-map-name="selectedLargeMapSub ?? ''"
              :areas="currentSubMapAreas"
              @update:area="onAreaChange"
              @delete:area="onDeleteArea"
              @add:area="onAddArea"
            />
          </template>

          <template v-else>
            <div class="empty-hint">{{ t('data.empty') }}</div>
          </template>
        </div>
      </section>
    </div>

    <div
      v-if="ctxMenu.visible"
      class="context-menu"
      :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
      @click.stop
      @contextmenu.prevent.stop
    >
      <button v-if="ctxMenu.canRename" class="ctx-item" @click="onCtxRename">{{ t('data.rename') }}</button>
      <button v-if="ctxMenu.canAddChild" class="ctx-item" @click="onCtxAddChild">{{ t('data.add') }}</button>
      <button v-if="ctxMenu.canAddRoot" class="ctx-item" @click="onCtxAddRoot">{{ t('data.add') }}</button>
      <button v-if="ctxMenu.canDelete" class="ctx-item danger" @click="onCtxDelete">{{ t('data.delete') }}</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, onDeactivated } from 'vue'
import { api } from '@/api'
import { useConfigStore, sortRegions } from '@/stores/config.store'
import { useI18n } from '@/composables/useI18n'
import { useSplitter } from '@/composables/useSplitter'
import { useRegionTree } from '@/composables/useRegionTree'
import { useRegionContextMenu } from '@/composables/useRegionContextMenu'
import { useRegionAutoSave } from '@/composables/useRegionAutoSave'
import { confirmDialog, promptDialog } from '@/composables/useModal'
import DataRegionEditPanel from '@/components/data/DataRegionEditPanel.vue'
import DataLargeMapEditPanel from '@/components/data/DataLargeMapEditPanel.vue'
import type { RegionConfig } from '@/types'
import type { RegionGroup, LargeMapArea } from '@/composables/useRegionTree'
import { stripCategoryPrefix, getCategoryPrefix } from '@/utils/regionName'

const configStore = useConfigStore()
const { t } = useI18n()

const regions = ref<RegionConfig[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const expanded = ref<Set<string>>(new Set())

const { width: leftWidth, onMouseDown: onSplitterMouseDown } = useSplitter({
  storageKey: 'data_left_width_v3',
  defaultWidth: 260,
  minWidth: 200,
  maxWidth: 500,
  containerSelector: '.data-body',
})

type PanelKey = 'base' | 'largeMapRoot' | 'largeMapSub'
const currentPanel = ref<PanelKey>('base')
const selectedBaseName = ref<string | null>(null)
const selectedLargeMapSub = ref<string | null>(null)
const selectedBaseRegion = ref<RegionConfig | null>(null)
const editTargetW = ref<number>(0)
const editTargetH = ref<number>(0)

const selectedKey = computed<string>(() => {
  if (currentPanel.value === 'base' && selectedBaseName.value) {
    return makeBaseKey(selectedBaseName.value)
  }
  if (currentPanel.value === 'largeMapSub' && selectedLargeMapSub.value) {
    return makeLargeMapSubKey(selectedLargeMapSub.value)
  }
  return ''
})

function makeBaseKey(name: string): string {
  return 'base:' + name
}
function makeLargeMapSubKey(name: string): string {
  return 'lmSub:' + name
}

const { baseTree, largeMapSubMaps, totalLargeMapCount, currentSubMapAreas } =
  useRegionTree(regions, selectedLargeMapSub)

// 数据操作：执行变更后刷新本地 regions、configStore 与当前选中区域
async function withRefresh<T>(fn: () => Promise<T>): Promise<T | undefined> {
  try {
    const result = await fn()
    await refresh()
    await configStore.load()
    await configStore.refreshCurrentRegion()
    return result
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  }
}

async function renameCategory(oldName: string, newName: string) {
  await withRefresh(async () => {
    const list = regions.value.filter((r) => r.category === oldName)
    const now = new Date().toISOString()
    for (const r of list) {
      const oldPrefix = `${oldName}-`
      const suffix = r.name.startsWith(oldPrefix) ? r.name.slice(oldPrefix.length) : r.name
      await api.upsertRegion({
        ...r,
        category: newName,
        name: `${newName}-${suffix}`,
        updated_at: now,
      })
    }
    if (selectedBaseName.value && selectedBaseRegion.value?.category === oldName) {
      selectedBaseName.value = null
      selectedBaseRegion.value = null
    }
  })
}

async function renameRegion(oldFullName: string, newShortName: string) {
  await withRefresh(async () => {
    const r = regions.value.find((x) => x.name === oldFullName)
    if (!r) return
    const prefix = getCategoryPrefix(oldFullName)
    const newName = prefix ? `${prefix}-${newShortName}` : newShortName
    await api.upsertRegion({ ...r, name: newName, updated_at: new Date().toISOString() })
    if (selectedBaseName.value === oldFullName) {
      selectedBaseName.value = newName
    }
  })
}

async function renameSubMap(oldName: string, newName: string) {
  await withRefresh(async () => {
    const list = regions.value.filter((r) => r.category === '大地图' && r.sub_map === oldName)
    const now = new Date().toISOString()
    for (const r of list) {
      await api.upsertRegion({ ...r, sub_map: newName, updated_at: now })
    }
    if (selectedLargeMapSub.value === oldName) {
      selectedLargeMapSub.value = newName
    }
  })
}

async function addBaseRegion(categoryName: string, shortName: string) {
  await withRefresh(async () => {
    const now = new Date().toISOString()
    const name = `${categoryName}-${shortName}`
    const empty = createEmptyBase(categoryName, name)
    await api.upsertRegion({ ...empty, created_at: now, updated_at: now })
    expanded.value.add(categoryName)
  })
}

async function addLargeMapArea(subMapName: string, areaName: string) {
  await withRefresh(async () => {
    const now = new Date().toISOString()
    const empty = createEmptyLargeMap(subMapName, areaName)
    await api.upsertRegion({ ...empty, created_at: now, updated_at: now })
    expanded.value.add('大地图')
  })
}

async function deleteCategory(categoryName: string) {
  await withRefresh(async () => {
    const list = regions.value.filter((r) => r.category === categoryName)
    for (const r of list) {
      await api.deleteRegion(r.name, r.aspect_ratio, r.scroll_mode)
    }
    if (selectedBaseRegion.value?.category === categoryName) {
      selectedBaseName.value = null
      selectedBaseRegion.value = null
    }
  })
}

async function deleteRegionByName(fullName: string) {
  await withRefresh(async () => {
    const r = regions.value.find((x) => x.name === fullName)
    if (!r) return
    await api.deleteRegion(r.name, r.aspect_ratio, r.scroll_mode)
    if (selectedBaseName.value === fullName) {
      selectedBaseName.value = null
      selectedBaseRegion.value = null
    }
  })
}

async function deleteSubMap(subMapName: string) {
  await withRefresh(async () => {
    const list = regions.value.filter((r) => r.category === '大地图' && r.sub_map === subMapName)
    for (const r of list) {
      await api.deleteRegion(r.name, r.aspect_ratio, r.scroll_mode)
    }
    if (selectedLargeMapSub.value === subMapName) {
      selectedLargeMapSub.value = null
    }
  })
}

const {
  ctxMenu,
  closeCtxMenu,
  onRootContextmenu,
  onCategoryContextmenu,
  onBaseRegionContextmenu,
  onLargeMapRootContextmenu,
  onLargeMapSubContextmenu,
  onCtxRename,
  onCtxAddChild,
  onCtxAddRoot,
  onCtxDelete,
} = useRegionContextMenu({
  renameCategory,
  renameRegion,
  renameSubMap,
  addBaseRegion,
  addLargeMapArea,
  deleteCategory,
  deleteRegionByName,
  deleteSubMap,
})

// 自动保存 + 3×3 推导
const { allCountsResult, flushPendingSave, setSkipAutoSave, rederive } =
  useRegionAutoSave({
    selectedBaseName,
    selectedBaseRegion,
    editTargetW,
    editTargetH,
    withRefresh,
  })

async function onAreaChange(area: LargeMapArea) {
  await withRefresh(async () => {
    const r = area.region
    const rows = Math.max(1, Math.min(100, Math.trunc(r.grid_rows) || 1))
    const cols = Math.max(1, Math.min(100, Math.trunc(r.grid_cols) || 1))
    await api.upsertRegion({
      ...r,
      grid_rows: rows,
      grid_cols: cols,
      updated_at: new Date().toISOString(),
    })
  })
}

async function onDeleteArea(area: LargeMapArea) {
  const ok = await confirmDialog({
    title: t('data.delete'),
    message: t('data.confirmDelete'),
    danger: true,
  })
  if (!ok) return
  await withRefresh(async () => {
    await api.deleteRegion(area.region.name, area.region.aspect_ratio, area.region.scroll_mode)
  })
}

async function onAddArea() {
  if (!selectedLargeMapSub.value) return
  const name = await promptDialog({
    title: t('data.promptAddArea'),
    placeholder: t('data.areaName'),
  })
  if (!name) return
  await addLargeMapArea(selectedLargeMapSub.value, name)
}

function onSelectBaseRegion(item: RegionGroup) {
  flushPendingSave()
  currentPanel.value = 'base'
  selectedBaseName.value = item.name
  selectedBaseRegion.value = { ...item.representative }
  selectedLargeMapSub.value = null
  setSkipAutoSave()
  editTargetW.value = item.targetW
  editTargetH.value = item.targetH
  void rederive()
}

function onSelectLargeMapSub(name: string) {
  flushPendingSave()
  currentPanel.value = 'largeMapSub'
  selectedLargeMapSub.value = name
  selectedBaseName.value = null
  selectedBaseRegion.value = null
}

function toggleExpand(name: string) {
  if (expanded.value.has(name)) {
    expanded.value.delete(name)
  } else {
    expanded.value.add(name)
  }
}

function createEmptyBase(category: string, name: string): RegionConfig {
  const now = new Date().toISOString()
  return {
    name, category,
    aspect_ratio: '16:9', scroll_mode: '0次',
    grid_rows: 2, grid_cols: 2,
    overlap_x: 0.001, overlap_y: 0.001,
    drag_x: 905, drag_y: 525,
    capture_region_x: 0.626, capture_region_y: 0.648,
    capture_offset_y: 0, template_ref: null,
    target_w: 0, target_h: 0,
    sub_map: null,
    created_at: now, updated_at: now,
  }
}

function createEmptyLargeMap(subMap: string, name: string): RegionConfig {
  const now = new Date().toISOString()
  return {
    name, category: '大地图',
    aspect_ratio: '16:9', scroll_mode: '0次',
    grid_rows: 10, grid_cols: 10,
    overlap_x: 0.001, overlap_y: 0.001,
    drag_x: 905, drag_y: 525,
    capture_region_x: 0.378, capture_region_y: 0.388,
    capture_offset_y: 0, template_ref: null,
    target_w: 0, target_h: 0,
    sub_map: subMap,
    created_at: now, updated_at: now,
  }
}

// 刷新：重新拉取 regions，初次加载时展开所有类别并自动选中首项
async function refresh() {
  loading.value = true
  error.value = null
  try {
    regions.value = sortRegions(await api.listRegions())
    if (expanded.value.size === 0) {
      expanded.value = new Set([
        ...baseTree.value.map((c) => c.name),
        '大地图',
      ])
    }
    if (!selectedBaseName.value && !selectedLargeMapSub.value && baseTree.value.length > 0) {
      const firstCat = baseTree.value[0]
      if (firstCat && firstCat.items.length > 0) {
        onSelectBaseRegion(firstCat.items[0])
      }
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

function onWindowClick() {
  closeCtxMenu()
}

onMounted(() => {
  void refresh()
  window.addEventListener('click', onWindowClick)
})

onDeactivated(() => {
  flushPendingSave()
  window.removeEventListener('click', onWindowClick)
})

onUnmounted(() => {
  flushPendingSave()
  window.removeEventListener('click', onWindowClick)
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
  position: relative;
}

.data-titlebar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
}

.data-title { font-size: 14px; font-weight: 600; margin: 0; color: var(--text-primary); }
.data-body { flex: 1; min-height: 0; display: flex; overflow: hidden; }

.sidebar {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border);
  background: var(--bg-secondary);
  overflow: hidden;
}

.list-scroll { flex: 1; overflow-y: auto; padding: 4px; }
.tree-group { margin-bottom: 2px; }

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

.tree-header:hover { background: var(--bg-tertiary); }
.tree-header.lv1 { padding-left: 4px; }

.tree-arrow { flex-shrink: 0; transition: transform 0.15s ease; fill: var(--text-muted); }
.tree-arrow.expanded { transform: rotate(90deg); }

.tree-name {
  font-size: 12px;
  color: var(--text-primary);
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tree-name.bold { font-weight: 600; }

.tree-count { font-size: 10px; color: var(--text-muted); background: var(--bg-tertiary); padding: 0 6px; border-radius: 8px; line-height: 14px; flex-shrink: 0; }

.tree-children { list-style: none; margin: 0; padding: 0; }

.tree-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px 6px 24px;
  cursor: pointer;
  border-radius: 3px;
  transition: background 0.12s ease;
}

.tree-item:hover { background: var(--bg-tertiary); }
.tree-item.active { background: var(--accent-light); }
.tree-item.lv2 { padding-left: 24px; }
.tree-meta { font-size: 11px; color: var(--text-secondary); font-variant-numeric: tabular-nums; flex-shrink: 0; }

.splitter {
  width: 6px;
  flex-shrink: 0;
  cursor: col-resize;
  background: var(--border);
  position: relative;
  transition: background 0.15s ease;
}

.splitter:hover,
.splitter:active { background: var(--accent); }

.splitter-handle {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 2px;
  height: 32px;
  background: var(--text-muted);
  border-radius: 1px;
  opacity: 0.5;
  pointer-events: none;
}

.edit-pane {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  overflow: hidden;
}

.edit-wrap { flex: 1; display: flex; flex-direction: column; overflow-y: auto; max-width: 650px; width: 100%; margin: 0 auto; }

.empty-hint {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  font-size: 13px;
  padding: 24px;
  text-align: center;
}

.error-hint { color: #e81123; }

.context-menu {
  position: fixed;
  z-index: 10000;
  min-width: 120px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.25);
  padding: 4px 0;
  user-select: none;
}

.ctx-item { display: block; width: 100%; padding: 6px 12px; background: transparent; border: none; color: var(--text-primary); font-size: 12px; font-family: inherit; text-align: left; cursor: pointer; transition: background 0.12s ease; }
.ctx-item:hover { background: var(--accent-light); color: var(--accent); }
.ctx-item.danger { color: #e81123; }
.ctx-item.danger:hover { background: #e81123; color: #fff; }
</style>
