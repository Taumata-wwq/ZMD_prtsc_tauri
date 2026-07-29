<template>
  <div class="log-panel">
    <!-- 顶部标题栏 -->
    <div class="log-header">
      <span class="log-title">日志</span>
      <div class="log-meta">
        <span class="log-count">{{ logs.length }} 条</span>
        <button
          type="button"
          class="clear-btn"
          :disabled="logs.length === 0"
          title="清空日志"
          @click="onClear"
        >
          清空
        </button>
      </div>
    </div>

    <!-- 日志列表 -->
    <div ref="scrollRef" class="log-body">
      <div v-if="logs.length === 0" class="empty">暂无日志</div>
      <div
        v-for="(item, idx) in logs"
        :key="`${item.timestamp}-${idx}`"
        class="log-line"
        :class="`level-${item.level}`"
      >
        <span class="log-time">[{{ formatTime(item.timestamp) }}]</span>
        <span class="log-msg">{{ item.message }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useCaptureStore } from '@/stores/capture.store'

const captureStore = useCaptureStore()
// 使用 storeToRefs 提取 ref，保证对 logs.value = []（替换引用）也保持响应式
const { logs } = storeToRefs(captureStore)

const scrollRef = ref<HTMLDivElement | null>(null)

/** 自动滚动到底部（watch logs.length 变化） */
watch(
  () => logs.value.length,
  () => {
    nextTick(() => {
      const el = scrollRef.value
      if (el) el.scrollTop = el.scrollHeight
    })
  },
)

// 初始化事件监听（store 幂等）
onMounted(() => {
  captureStore.init().catch((e) => {
    console.error('[LogPanel] captureStore.init 失败:', e)
  })
})

function onClear() {
  captureStore.clearLogs()
}

/** 格式化时间戳（保留 HH:MM:SS） */
function formatTime(timestamp: string): string {
  if (!timestamp) return ''
  // 兼容 ISO 字符串与已经格式化过的字符串
  const date = new Date(timestamp)
  if (Number.isNaN(date.getTime())) return timestamp
  const h = String(date.getHours()).padStart(2, '0')
  const m = String(date.getMinutes()).padStart(2, '0')
  const s = String(date.getSeconds()).padStart(2, '0')
  return `${h}:${m}:${s}`
}
</script>

<style scoped>
.log-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--bg-secondary);
  overflow: hidden;
}

.log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 10px;
  background: var(--bg-tertiary);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.log-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 0.5px;
}

.log-meta {
  display: flex;
  align-items: center;
  gap: 8px;
}

.log-count {
  font-size: 11px;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
}

.clear-btn {
  padding: 2px 8px;
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-secondary);
  border-radius: 3px;
  font-size: 11px;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease;
}

.clear-btn:hover:not(:disabled) {
  background: var(--btn-hover-bg);
  color: var(--text-primary);
  border-color: var(--accent);
}

.clear-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.log-body {
  flex: 1;
  overflow-y: auto;
  padding: 6px 10px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 12px;
  line-height: 1.55;
  min-height: 0;
}

.empty {
  color: var(--text-muted);
  text-align: center;
  padding: 20px 0;
  font-size: 12px;
}

.log-line {
  display: flex;
  gap: 6px;
  white-space: pre-wrap;
  word-break: break-all;
  padding: 1px 0;
}

.log-time {
  color: var(--text-secondary);
  flex-shrink: 0;
}

.log-msg {
  color: var(--text-primary);
  white-space: pre-wrap;
}

/* 按 level 着色 */
.level-info .log-msg {
  color: var(--text-primary);
}

.level-warn .log-msg {
  color: #f0a020;
}

.level-error .log-msg {
  color: #e81123;
}
</style>
