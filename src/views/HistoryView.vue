<template>
  <div class="history-view">
    <!-- 顶部工具栏 -->
    <header class="toolbar">
      <h2 class="title">{{ t('history.title') }}</h2>
      <div class="toolbar-actions">
        <span v-if="historyStore.loading" class="loading-hint">{{ t('history.loading') }}</span>
        <span v-else-if="historyStore.error" class="error-hint" :title="historyStore.error">{{ t('history.loadFailed') }}</span>
        <button class="btn" type="button" :disabled="historyStore.loading" @click="refresh">
          {{ t('history.refresh') }}
        </button>
        <!-- 清空历史记录按钮 -->
        <button
          class="btn btn-danger"
          type="button"
          :disabled="historyStore.loading || historyStore.sessions.length === 0"
          @click="onClear"
        >
          {{ t('history.clear') }}
        </button>
      </div>
    </header>

    <!-- 主体：左右分栏（左侧宽度 250px，可拖动调节） -->
    <div class="history-body">
      <!-- 左侧：会话列表 -->
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
              <span class="status-badge" :class="statusClass(s.status)">{{ s.status }}</span>
            </div>
            <div class="session-row session-row-meta">
              <span class="meta-region" :title="s.region ?? ''">{{ s.region || t('history.unknownRegion') }}</span>
              <span class="meta-sep">·</span>
              <span class="meta-scroll">{{ s.scroll_mode || t('history.defaultMode') }}</span>
            </div>
            <div class="session-row session-row-foot">
              <span class="meta-shots">{{ s.total_shots ?? 0 }} {{ t('history.shots') }}</span>
              <span v-if="s.original_path" class="meta-thumb" :title="s.original_path">
                {{ basename(s.original_path) }}
              </span>
            </div>
          </li>
        </ul>
      </aside>

      <!-- 可拖动分割线 -->
      <div
        class="splitter"
        @mousedown="onSplitterMouseDown"
      >
        <div class="splitter-handle" />
      </div>

      <!-- 右侧：详情面板 -->
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

            <dt>{{ t('history.grid') }}</dt>
            <dd>
              <template v-if="selected.grid_rows != null && selected.grid_cols != null">
                {{ selected.grid_rows }} × {{ selected.grid_cols }}
              </template>
              <template v-else>—</template>
            </dd>

            <dt>{{ t('history.totalShots') }}</dt>
            <dd>{{ selected.total_shots ?? '—' }}</dd>

            <dt>{{ t('history.status') }}</dt>
            <dd>
              <span class="status-badge" :class="statusClass(selected.status)">
                {{ selected.status }}
              </span>
            </dd>

            <dt>{{ t('history.outputFormat') }}</dt>
            <dd>{{ selected.output_format || '—' }}</dd>

            <dt>{{ t('history.jpgQuality') }}</dt>
            <dd>{{ selected.jpg_quality ?? '—' }}</dd>

            <dt>{{ t('history.originalPath') }}</dt>
            <dd class="path-cell" :title="selected.original_path ?? ''">
              <span class="path-text">{{ selected.original_path || '—' }}</span>
            </dd>

            <dt>{{ t('history.exportedPath') }}</dt>
            <dd class="path-cell" :title="selected.exported_path ?? ''">
              <span class="path-text">{{ selected.exported_path || '—' }}</span>
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
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useHistoryStore } from '@/stores/history.store'
import { useI18n } from '@/composables/useI18n'
import { useSplitter } from '@/composables/useSplitter'
import type { CaptureSession } from '@/types'

const historyStore = useHistoryStore()
const { t } = useI18n()

// -----------------------------------------------------------------------------
// 选中态
// -----------------------------------------------------------------------------
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

// -----------------------------------------------------------------------------
// 加载与刷新
// -----------------------------------------------------------------------------
async function refresh() {
  try {
    await historyStore.load()
    // 若当前选中项已不存在（被刷新掉），重置选中
    if (
      selectedId.value !== null &&
      !historyStore.sessions.some((s) => (s.id ?? s.started_at) === selectedId.value)
    ) {
      selectedId.value = null
    }
  } catch {
    // 错误已写入 historyStore.error，模板已展示
  }
}

/** 清空所有历史记录 */
async function onClear() {
  if (!confirm(t('history.confirmClear'))) return
  try {
    await historyStore.clear()
    selectedId.value = null
  } catch {
    // 错误已写入 historyStore.error，模板已展示
  }
}

onMounted(() => {
  // 进入视图时自动加载
  void refresh()
})

// -----------------------------------------------------------------------------
// 可拖动分割线（逻辑由 useSplitter composable 提供）
//
// 设计：
//   - 默认左侧 250px，调整范围 180-500px
//   - 拖拽时实时更新 leftWidth，结束时持久化到 localStorage
//   - mounted/unmount/deactivate 时自动恢复与清理
// -----------------------------------------------------------------------------
const { width: leftWidth, onMouseDown: onSplitterMouseDown } = useSplitter({
  storageKey: 'history_left_width',
  defaultWidth: 250,
  minWidth: 180,
  maxWidth: 500,
  containerSelector: '.history-body',
})

// -----------------------------------------------------------------------------
// 工具函数：时间格式化（YYYY-MM-DD HH:mm:ss）
// -----------------------------------------------------------------------------
function formatDateTime(iso?: string | null): string {
  if (!iso) return ''
  const d = new Date(iso)
  if (isNaN(d.getTime())) return iso
  const pad = (n: number) => n.toString().padStart(2, '0')
  return (
    `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ` +
    `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  )
}

// -----------------------------------------------------------------------------
// 工具函数：从路径中提取文件名
// -----------------------------------------------------------------------------
function basename(p?: string | null): string {
  if (!p) return ''
  const parts = p.split(/[\\/]/)
  return parts[parts.length - 1] ?? p
}

// -----------------------------------------------------------------------------
// 工具函数：状态徽章 class
// -----------------------------------------------------------------------------
function statusClass(status: string): string {
  switch (status) {
    case 'completed':
      return 'status-completed'
    case 'discarded':
      return 'status-discarded'
    case 'interrupted':
      return 'status-interrupted'
    case 'error':
      return 'status-error'
    case 'capturing':
      return 'status-capturing'
    default:
      return 'status-unknown'
  }
}

// -----------------------------------------------------------------------------
// 打开文件位置
// -----------------------------------------------------------------------------
async function openLocation(path?: string | null) {
  if (!path) return
  try {
    const opener = await import('@tauri-apps/plugin-opener')
    if (typeof opener.revealItemInDir === 'function') {
      await opener.revealItemInDir(path)
      return
    }
    if (typeof opener.openPath === 'function') {
      await opener.openPath(path)
      return
    }
  } catch (e) {
    console.warn('[HistoryView] plugin-opener 不可用，尝试 Rust open_path:', e)
  }
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('open_path', { path })
  } catch (e) {
    console.warn('[HistoryView] Rust open_path 不可用:', e)
    alert('功能待实现：打开文件位置需要 plugin-opener（已声明于 package.json，请执行 npm install）或 Rust open_path 命令。')
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

/* --- 顶部工具栏 --- */
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

.title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

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

.btn:hover:not(:disabled) {
  background: var(--btn-hover-bg);
  border-color: var(--accent);
}

.btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn-danger {
  background: #e81123;
  color: #ffffff;
  border-color: #e81123;
}

.btn-danger:hover:not(:disabled) {
  background: #c50f1f;
  border-color: #c50f1f;
}

.loading-hint {
  font-size: 12px;
  color: var(--text-secondary);
}

.error-hint {
  font-size: 12px;
  color: #ff6b6b;
  cursor: help;
}

/* --- 主体：左右分栏 --- */
.history-body {
  flex: 1;
  display: flex;
  min-height: 0;
}

/* --- 左侧会话列表（宽度由 leftWidth 控制） --- */
.session-pane {
  flex-shrink: 0;
  border-right: 1px solid var(--border);
  background: var(--bg-secondary);
  overflow-y: auto;
  overflow-x: hidden;
}

.session-list {
  list-style: none;
  margin: 0;
  padding: 4px;
}

.session-item {
  padding: 8px 10px;
  border-radius: 3px;
  cursor: pointer;
  transition: background 0.12s ease;
  border: 1px solid transparent;
}

.session-item:hover {
  background: var(--bg-tertiary);
}

.session-item.active {
  background: var(--accent-light);
  border-color: var(--accent);
}

.session-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.session-row-time {
  justify-content: space-between;
}

.session-row-meta {
  margin-top: 4px;
  color: var(--text-secondary);
  font-size: 12px;
}

.session-row-foot {
  margin-top: 4px;
  color: var(--text-muted);
  font-size: 11px;
  justify-content: space-between;
}

.session-time {
  font-size: 12px;
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

.meta-region {
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 160px;
}

.meta-sep {
  color: var(--text-muted);
}

.meta-shots {
  flex-shrink: 0;
}

.meta-thumb {
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 140px;
  font-style: italic;
}

/* --- 状态徽章 --- */
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

.status-completed {
  background: rgba(76, 175, 80, 0.18);
  color: #5ecb6e;
  border: 1px solid rgba(76, 175, 80, 0.4);
}

.status-discarded {
  background: rgba(150, 150, 150, 0.18);
  color: var(--text-secondary);
  border: 1px solid rgba(150, 150, 150, 0.4);
}

.status-interrupted {
  background: rgba(255, 152, 0, 0.18);
  color: #ffb74d;
  border: 1px solid rgba(255, 152, 0, 0.4);
}

.status-error {
  background: rgba(244, 67, 54, 0.18);
  color: #ff6b6b;
  border: 1px solid rgba(244, 67, 54, 0.4);
}

.status-capturing {
  background: rgba(0, 181, 229, 0.18);
  color: var(--accent);
  border: 1px solid rgba(0, 181, 229, 0.4);
}

.status-unknown {
  background: rgba(150, 150, 150, 0.18);
  color: var(--text-secondary);
  border: 1px solid rgba(150, 150, 150, 0.4);
}

/* --- 可拖动分割线 --- */
.splitter {
  width: 6px;
  flex-shrink: 0;
  cursor: col-resize;
  background: var(--border);
  position: relative;
  transition: background 0.15s ease;
}

.splitter:hover,
.splitter:active {
  background: var(--accent);
}

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
.splitter:active .splitter-handle {
  background: #ffffff;
  opacity: 0.8;
}

/* --- 右侧详情面板 --- */
.detail-pane {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
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
}

/* --- 详情内容 --- */
.detail-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.kv-grid {
  display: grid;
  grid-template-columns: 100px 1fr;
  gap: 8px 12px;
  font-size: 13px;
}

.kv-grid dt {
  color: var(--text-secondary);
  font-weight: normal;
}

.kv-grid dd {
  color: var(--text-primary);
  word-break: break-all;
}

.path-cell {
  display: flex;
  align-items: center;
  min-width: 0;
}

.path-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: "Consolas", "Monaco", monospace;
  font-size: 12px;
}

.detail-actions {
  margin-top: 20px;
  display: flex;
  gap: 8px;
}

.detail-actions .btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
