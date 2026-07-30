<template>
  <div class="history-view">
    <header class="toolbar">
      <h2 class="title">{{ t('history.title') }}</h2>
      <div class="toolbar-actions">
        <span v-if="historyStore.loading" class="loading-hint">{{ t('history.loading') }}</span>
        <span v-else-if="historyStore.error" class="error-hint" :title="historyStore.error">{{ t('history.loadFailed') }}</span>
      </div>
    </header>

    <!-- 主体：左侧会话列表 + 右侧详情面板（左侧宽度 250px，可拖动调节） -->
    <div class="history-body">
      <aside
        class="session-pane"
        :style="{ width: leftWidth + 'px', flexBasis: leftWidth + 'px' }"
      >
        <div v-if="historyStore.loading && historyStore.sessions.length === 0" class="empty-state">
          {{ t('history.loading') }}
        </div>
        <div v-else-if="historyStore.sessions.length === 0" class="empty-state">
          {{ t('history.empty') }}
        </div>
        <ul v-else class="session-list">
          <li
            v-for="s in historyStore.sessions"
            :key="s.id ?? s.started_at"
            class="session-item"
            :class="{ active: selectedId === (s.id ?? s.started_at) }"
            @click="onSelect(s)"
          >
            <div class="session-row session-row-time">
              <span class="session-time">{{ formatDateTime(s.started_at) }}</span>
              <span class="status-badge" :class="statusClass(s.status)">{{ statusText(s.status) }}</span>
            </div>
            <div class="session-row session-row-meta">
              <span class="meta-region" :title="s.region ?? ''">{{ s.region || t('history.unknownRegion') }}</span>
              <span class="meta-sep">·</span>
              <span class="meta-scroll">{{ s.scroll_mode || t('history.defaultMode') }}</span>
            </div>
          </li>
        </ul>
      </aside>

      <div class="splitter" @mousedown="onSplitterMouseDown">
        <div class="splitter-handle" />
      </div>

      <section class="detail-pane">
        <div v-if="!selected" class="empty-state">{{ t('history.selectSession') }}</div>
        <div v-else class="detail-content selectable">
          <dl class="kv-grid">
            <dt>{{ t('history.startTime') }}</dt>
            <dd>{{ formatDateTime(selected.started_at) || '—' }}</dd>

            <dt>{{ t('history.endTime') }}</dt>
            <dd>{{ selected.finished_at ? formatDateTime(selected.finished_at) : '—' }}</dd>

            <dt>{{ t('history.region') }}</dt>
            <dd>{{ selected.region || '—' }}</dd>

            <dt>{{ t('history.scrollMode') }}</dt>
            <dd>{{ selected.scroll_mode || '—' }}</dd>

            <dt>{{ t('history.status') }}</dt>
            <dd>
              <span class="status-badge" :class="statusClass(selected.status)">
                {{ statusText(selected.status) }}
              </span>
            </dd>

            <dt>{{ t('history.originalPath') }}</dt>
            <dd class="path-cell" :title="selected.original_path ?? ''">
              <span
                v-if="selected.original_path"
                class="path-text path-link"
                @click="openImage(selected.original_path)"
              >{{ selected.original_path }}</span>
              <span v-else>—</span>
            </dd>

            <dt>{{ t('history.exportedPath') }}</dt>
            <dd class="path-cell">
              <div v-if="exportedPathList.length" class="path-list">
                <span
                  v-for="(p, i) in exportedPathList"
                  :key="i"
                  class="path-text path-link"
                  :title="p"
                  @click="openImage(p)"
                >{{ p }}</span>
              </div>
              <span v-else>—</span>
            </dd>
          </dl>

          <div class="detail-actions">
            <button
              type="button"
              class="btn"
              :disabled="!selected.exported_path"
              @click="openLocation(selected.exported_path)"
            >
              {{ t('history.openLocation') }}
            </button>
            <button
              type="button"
              class="btn"
              :disabled="!selected.original_path"
              @click="openLocation(selected.original_path)"
            >
              {{ t('history.openOriginalLocation') }}
            </button>
            <button
              type="button"
              class="btn btn-primary"
              :disabled="!selected.original_path"
              @click="onReedit"
            >
              {{ t('history.reedit') }}
            </button>
            <button
              type="button"
              class="btn btn-danger"
              @click="showDeleteDialog"
            >
              {{ t('history.deleteRecord') }}
            </button>
          </div>

          <div v-if="thumbUrl" class="detail-thumbnail">
            <img :src="thumbUrl" :alt="t('history.originalPath')" class="thumbnail-img" />
          </div>
        </div>
      </section>
    </div>

    <div v-if="isOpen" class="modal-overlay" @click.self="closeDeleteDialog">
      <div class="modal-dialog">
        <h3 class="modal-title">{{ t('history.deleteConfirmTitle') }}</h3>
        <p class="modal-message">{{ t('history.deleteConfirmMessage') }}</p>
        <div v-if="hasOriginalPath || hasExportedPath" class="modal-checkboxes">
          <label v-if="hasOriginalPath" class="checkbox-label">
            <input type="checkbox" v-model="deleteOriginal" />
            <span>{{ t('history.deleteCurrentOriginal') }}</span>
          </label>
          <label v-if="hasExportedPath" class="checkbox-label">
            <input type="checkbox" v-model="deleteScreenshot" />
            <span>{{ t('history.deleteCurrentScreenshot') }}</span>
          </label>
        </div>
        <div class="modal-actions">
          <button type="button" class="btn" @click="closeDeleteDialog">{{ t('common.cancel') }}</button>
          <button type="button" class="btn btn-danger" :disabled="isConfirming" @click="confirmDelete">
            {{ isConfirming ? '...' : t('common.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onActivated, ref, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { useHistoryStore } from '@/stores/history.store'
import { useCaptureStore } from '@/stores/capture.store'
import { useUiStore } from '@/stores/ui.store'
import { useI18n } from '@/composables/useI18n'
import { useSplitter } from '@/composables/useSplitter'
import { useConfirmDialog } from '@/composables/useConfirmDialog'
import { openPath, revealItemInDir } from '@/utils/opener'
import { formatTimestamp } from '@/utils/datetime'
import type { CaptureSession } from '@/types'

const historyStore = useHistoryStore()
const captureStore = useCaptureStore()
const uiStore = useUiStore()
const { t } = useI18n()

const selectedId = ref<number | string | null>(null)

const selected = computed<CaptureSession | null>(() => {
  if (selectedId.value === null) return null
  return (
    historyStore.sessions.find((s) => (s.id ?? s.started_at) === selectedId.value) ?? null
  )
})

function onSelect(s: CaptureSession) {
  selectedId.value = s.id ?? s.started_at
}

const { isOpen, isConfirming, open, close } = useConfirmDialog()
const deleteOriginal = ref(false)
const deleteScreenshot = ref(false)

/** 当前选中会话的截图路径列表（exported_path 换行分隔） */
const exportedPathList = computed<string[]>(() => {
  const raw = selected.value?.exported_path ?? ''
  if (!raw) return []
  return raw.split('\n').map((s) => s.trim()).filter((s) => s.length > 0)
})

const hasExportedPath = computed(() => exportedPathList.value.length > 0)
const hasOriginalPath = computed(() => !!selected.value?.original_path)

function showDeleteDialog() {
  deleteOriginal.value = false
  deleteScreenshot.value = false
  open()
}

function closeDeleteDialog() {
  close()
}

async function confirmDelete() {
  const s = selected.value
  if (!s?.id) return
  isConfirming.value = true
  try {
    await historyStore.deleteSession(s.id, deleteOriginal.value, deleteScreenshot.value)
    // 删除后自动选中首条记录
    const list = historyStore.sessions
    selectedId.value = list.length > 0 ? (list[0].id ?? list[0].started_at) : null
  } catch (e) {
    console.error('[HistoryView] 删除记录失败:', e)
  } finally {
    isConfirming.value = false
    close()
  }
}

async function openImage(path: string) {
  if (!path) return
  try {
    await openPath(path)
  } catch (e) {
    console.error('[HistoryView] 打开图片失败:', e)
  }
}

/** 当前选中会话的缩略图 URL（优先 thumbnail_path，回退到原图） */
const thumbUrl = computed<string>(() => {
  const s = selected.value
  if (!s) return ''
  const thumbPath = s.thumbnail_path || s.original_path || ''
  if (!thumbPath) return ''
  return convertFileSrc(thumbPath.replace(/\\/g, '/'))
})

// 会话列表变化时保持选中项有效：选中项不存在则回退到首条；无选中且有数据则选首条
watch(
  () => historyStore.sessions,
  (sessions) => {
    if (selectedId.value !== null) {
      if (!sessions.some((s) => (s.id ?? s.started_at) === selectedId.value)) {
        selectedId.value = sessions.length > 0
          ? (sessions[0].id ?? sessions[0].started_at)
          : null
      }
    } else if (sessions.length > 0) {
      selectedId.value = sessions[0].id ?? sessions[0].started_at
    }
  },
)

onMounted(() => {
  void historyStore.refresh()
})

// keep-alive 激活时自动刷新一次
onActivated(() => {
  void historyStore.refresh()
})

const { width: leftWidth, onMouseDown: onSplitterMouseDown } = useSplitter({
  storageKey: 'history_left_width',
  defaultWidth: 250,
  minWidth: 200,
  maxWidth: 500,
  containerSelector: '.history-body',
})

function formatDateTime(iso?: string | null): string {
  if (!iso) return ''
  const d = new Date(iso)
  if (isNaN(d.getTime())) return iso
  return formatTimestamp(d, 'YYYY-MM-DD HH:mm:ss')
}

function statusClass(status: string): string {
  switch (status) {
    case 'completed': return 'status-completed'
    case 'interrupted': return 'status-interrupted'
    case 'error': return 'status-error'
    case 'capturing': return 'status-capturing'
    default: return 'status-unknown'
  }
}

function statusText(status: string): string {
  switch (status) {
    case 'completed': return t('history.status.completed')
    case 'interrupted': return t('history.status.interrupted')
    case 'error': return t('history.status.error')
    case 'capturing': return t('history.status.capturing')
    default: return t('history.status.unknown')
  }
}

async function onReedit() {
  if (!selected.value) return
  try {
    await captureStore.loadFromSession(selected.value)
    uiStore.setView('capture')
  } catch (e) {
    console.error('[HistoryView] 重新编辑失败:', e)
  }
}

async function openLocation(path?: string | null) {
  if (!path) return
  try {
    await revealItemInDir(path)
  } catch (e) {
    console.error('[HistoryView] 打开文件位置失败:', e)
  }
}
</script>

<style scoped>
.history-view {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  background: var(--bg-primary);
  overflow: hidden;
}

.toolbar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-secondary);
}

.title { font-size: 14px; font-weight: 600; color: var(--text-primary); margin: 0; }
.toolbar-actions { display: flex; align-items: center; gap: 8px; }

.btn {
  height: 28px;
  padding: 0 10px;
  border: 1px solid var(--border);
  border-radius: 3px;
  background: var(--bg-tertiary);
  color: var(--text-primary);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
  white-space: nowrap;
}

.btn:hover:not(:disabled) { background: var(--btn-hover-bg); border-color: var(--accent); }
.btn:disabled { opacity: 0.4; cursor: not-allowed; }

.btn-primary {
  background: var(--accent);
  color: #ffffff;
  border-color: var(--accent);
}

.btn-primary:hover:not(:disabled) { background: var(--accent-hover); border-color: var(--accent-hover); }

.loading-hint { font-size: 12px; color: var(--text-secondary); }
.error-hint { font-size: 12px; color: #ff6b6b; cursor: help; }

.history-body { flex: 1; display: flex; min-height: 0; }

.session-pane {
  flex-shrink: 0;
  border-right: 1px solid var(--border);
  background: var(--bg-secondary);
  overflow-y: auto;
  overflow-x: hidden;
}

.session-list { list-style: none; margin: 0; padding: 4px; }

.session-item {
  padding: 8px 10px;
  border-radius: 3px;
  cursor: pointer;
  transition: background 0.12s ease;
  border: 1px solid transparent;
}

.session-item:hover { background: var(--bg-tertiary); }
.session-item.active { background: var(--accent-light); border-color: var(--accent); }

.session-row { display: flex; align-items: center; gap: 6px; }
.session-row-time { justify-content: space-between; }
.session-row-meta { margin-top: 4px; color: var(--text-secondary); font-size: 12px; }

.session-time { font-size: 12px; color: var(--text-primary); font-variant-numeric: tabular-nums; }

.meta-region {
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 160px;
}

.meta-sep { color: var(--text-muted); }

.status-badge {
  display: inline-block;
  padding: 1px 6px;
  border-radius: 2px;
  font-size: 10px;
  font-weight: 600;
  line-height: 1.4;
  text-transform: uppercase;
  letter-spacing: 0.3px;
  flex-shrink: 0;
}

.status-completed { background: rgba(76, 175, 80, 0.18); color: #5ecb6e; border: 1px solid rgba(76, 175, 80, 0.4); }
.status-interrupted { background: rgba(255, 152, 0, 0.18); color: #ffb74d; border: 1px solid rgba(255, 152, 0, 0.4); }
.status-error { background: rgba(244, 67, 54, 0.18); color: #ff6b6b; border: 1px solid rgba(244, 67, 54, 0.4); }
.status-capturing { background: rgba(0, 181, 229, 0.18); color: var(--accent); border: 1px solid rgba(0, 181, 229, 0.4); }
.status-unknown { background: rgba(150, 150, 150, 0.18); color: var(--text-secondary); border: 1px solid rgba(150, 150, 150, 0.4); }

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

.splitter:hover .splitter-handle,
.splitter:active .splitter-handle { background: #ffffff; opacity: 0.8; }

.detail-pane {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  background: var(--bg-primary);
  overflow: hidden;
}

.empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  font-size: 13px;
  padding: 24px;
  text-align: center;
  max-width: 650px;
  width: 100%;
}

.detail-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  max-width: 650px;
  width: 100%;
  box-sizing: border-box;
}

.kv-grid { display: grid; grid-template-columns: 100px 1fr; gap: 8px 12px; font-size: 13px; }
.kv-grid dt { color: var(--text-secondary); font-weight: normal; }
.kv-grid dd { color: var(--text-primary); word-break: break-all; }

.path-cell { display: flex; align-items: center; min-width: 0; }

.path-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: "Consolas", "Monaco", monospace;
  font-size: 12px;
}

.path-link { cursor: pointer; color: var(--accent); }
.path-link:hover { text-decoration: underline; }
.path-list { display: flex; flex-direction: column; gap: 2px; }

.detail-actions { margin-top: 16px; display: flex; gap: 8px; flex-wrap: wrap; }

.detail-thumbnail {
  margin-top: 16px;
  display: flex;
  justify-content: center;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 3px;
  padding: 8px;
  overflow: hidden;
}

.thumbnail-img { max-width: 100%; max-height: 360px; object-fit: contain; display: block; }
.detail-actions .btn:disabled { opacity: 0.5; cursor: not-allowed; }

.btn-danger { background: #e81123; color: #ffffff; border-color: #e81123; }
.btn-danger:hover:not(:disabled) { background: #c50f1f; border-color: #c50f1f; color: #ffffff; }

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-dialog {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 20px;
  min-width: 360px;
  max-width: 460px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

.modal-title { margin: 0 0 12px 0; font-size: 14px; font-weight: 600; color: var(--text-primary); }
.modal-message { margin: 0 0 16px 0; font-size: 13px; color: var(--text-secondary); line-height: 1.5; }
.modal-checkboxes { display: flex; flex-direction: column; gap: 10px; margin-bottom: 18px; }

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-primary);
  cursor: pointer;
}

.checkbox-label input[type="checkbox"] { margin: 0; cursor: pointer; }
.modal-actions { display: flex; justify-content: flex-end; gap: 8px; }
</style>
