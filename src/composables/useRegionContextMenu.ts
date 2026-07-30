import { ref } from 'vue'
import { useI18n } from '@/composables/useI18n'
import { confirmDialog, promptDialog } from '@/composables/useModal'
import { stripCategoryPrefix } from '@/utils/regionName'

/** 右键菜单触发的层级 */
export type CtxMenuLevel =
  | 'root'
  | 'baseCategory'
  | 'baseRegion'
  | 'largeMapRoot'
  | 'largeMapSub'

/** 右键菜单状态 */
export interface CtxMenu {
  visible: boolean
  x: number
  y: number
  canRename: boolean
  canAddChild: boolean
  canAddRoot: boolean
  canDelete: boolean
  level: CtxMenuLevel
  categoryName?: string
  regionName?: string
  subMapName?: string
}

/** 右键菜单触发的数据操作回调集合 */
export interface RegionContextMenuActions {
  renameCategory: (oldName: string, newName: string) => Promise<void>
  renameRegion: (oldFullName: string, newShortName: string) => Promise<void>
  renameSubMap: (oldName: string, newName: string) => Promise<void>
  addBaseRegion: (categoryName: string, shortName: string) => Promise<void>
  addLargeMapArea: (subMapName: string, areaName: string) => Promise<void>
  deleteCategory: (categoryName: string) => Promise<void>
  deleteRegionByName: (fullName: string) => Promise<void>
  deleteSubMap: (subMapName: string) => Promise<void>
}

/**
 * 右键菜单状态机：菜单显示/隐藏、5 个触发入口与 4 个动作处理器
 * 处理器内部直接调用 promptDialog/confirmDialog，数据操作通过 actions 注入。
 */
export function useRegionContextMenu(actions: RegionContextMenuActions) {
  const { t } = useI18n()

  const ctxMenu = ref<CtxMenu>({
    visible: false,
    x: 0,
    y: 0,
    canRename: false,
    canAddChild: false,
    canAddRoot: false,
    canDelete: false,
    level: 'root',
  })

  function showCtxMenu(e: MouseEvent, partial: Partial<CtxMenu>) {
    ctxMenu.value = {
      visible: true,
      x: e.clientX,
      y: e.clientY,
      canRename: false,
      canAddChild: false,
      canAddRoot: false,
      canDelete: false,
      level: 'root',
      ...partial,
    }
  }

  function closeCtxMenu() {
    ctxMenu.value.visible = false
  }

  function onRootContextmenu(e: MouseEvent) {
    showCtxMenu(e, { level: 'root', canAddRoot: true })
  }

  function onCategoryContextmenu(e: MouseEvent, categoryName: string) {
    showCtxMenu(e, { level: 'baseCategory', categoryName, canRename: true, canAddChild: true, canDelete: true })
  }

  function onBaseRegionContextmenu(e: MouseEvent, regionName: string) {
    showCtxMenu(e, { level: 'baseRegion', regionName, canRename: true, canDelete: true })
  }

  function onLargeMapRootContextmenu(e: MouseEvent) {
    showCtxMenu(e, { level: 'largeMapRoot', canAddChild: true })
  }

  function onLargeMapSubContextmenu(e: MouseEvent, subMapName: string) {
    showCtxMenu(e, { level: 'largeMapSub', subMapName, canRename: true, canAddChild: true, canDelete: true })
  }

  async function onCtxRename() {
    const ctx = ctxMenu.value
    closeCtxMenu()
    if (ctx.level === 'baseCategory' && ctx.categoryName) {
      const newName = await promptDialog({ title: t('data.promptRenameCategory'), defaultValue: ctx.categoryName })
      if (!newName || newName === ctx.categoryName) return
      await actions.renameCategory(ctx.categoryName, newName)
    } else if (ctx.level === 'baseRegion' && ctx.regionName) {
      const oldPrefix = stripCategoryPrefix(ctx.regionName)
      const newName = await promptDialog({ title: t('data.promptRenameRegion'), defaultValue: oldPrefix })
      if (!newName || newName === oldPrefix) return
      await actions.renameRegion(ctx.regionName, newName)
    } else if (ctx.level === 'largeMapSub' && ctx.subMapName) {
      const newName = await promptDialog({ title: t('data.promptRenameSubMap'), defaultValue: ctx.subMapName })
      if (!newName || newName === ctx.subMapName) return
      await actions.renameSubMap(ctx.subMapName, newName)
    }
  }

  async function onCtxAddChild() {
    const ctx = ctxMenu.value
    closeCtxMenu()
    if (ctx.level === 'baseCategory' && ctx.categoryName) {
      const name = await promptDialog({ title: t('data.promptAddRegion'), placeholder: t('data.name') })
      if (!name) return
      await actions.addBaseRegion(ctx.categoryName, name)
    } else if (ctx.level === 'largeMapRoot') {
      const smName = await promptDialog({ title: t('data.promptAddSubMap'), placeholder: t('data.subMap') })
      if (!smName) return
      const areaName = await promptDialog({ title: t('data.promptAddArea'), placeholder: t('data.areaName') })
      if (!areaName) return
      await actions.addLargeMapArea(smName, areaName)
    } else if (ctx.level === 'largeMapSub' && ctx.subMapName) {
      const name = await promptDialog({ title: t('data.promptAddArea'), placeholder: t('data.areaName') })
      if (!name) return
      await actions.addLargeMapArea(ctx.subMapName, name)
    }
  }

  async function onCtxAddRoot() {
    closeCtxMenu()
    const catName = await promptDialog({ title: t('data.promptAddCategory'), placeholder: t('data.category') })
    if (!catName) return
    const regionName = await promptDialog({ title: t('data.promptAddRegion'), placeholder: t('data.name') })
    if (!regionName) return
    await actions.addBaseRegion(catName, regionName)
  }

  async function onCtxDelete() {
    const ctx = ctxMenu.value
    closeCtxMenu()
    const ok = await confirmDialog({ title: t('data.delete'), message: t('data.confirmDelete'), danger: true })
    if (!ok) return
    if (ctx.level === 'baseCategory' && ctx.categoryName) {
      await actions.deleteCategory(ctx.categoryName)
    } else if (ctx.level === 'baseRegion' && ctx.regionName) {
      await actions.deleteRegionByName(ctx.regionName)
    } else if (ctx.level === 'largeMapSub' && ctx.subMapName) {
      await actions.deleteSubMap(ctx.subMapName)
    }
  }

  return {
    ctxMenu,
    showCtxMenu,
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
  }
}
