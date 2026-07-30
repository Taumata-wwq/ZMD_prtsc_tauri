<template>
  <div v-if="!region" class="empty-hint">{{ t('data.empty') }}</div>
  <div v-else class="edit-content">
    <header class="edit-header">
      <h3 class="edit-title">{{ stripCategoryPrefix(region.name) }}</h3>
      <span class="edit-subtitle">{{ region.category }}</span>
    </header>

    <div class="form-row">
      <label class="form-label">Target</label>
      <div class="target-group">
        <input
          v-model.number="targetW"
          type="number"
          class="form-input target-input"
          min="908"
          max="16000"
          placeholder="W"
        />
        <span class="target-sep">×</span>
        <input
          v-model.number="targetH"
          type="number"
          class="form-input target-input"
          min="528"
          max="16000"
          placeholder="H"
        />
      </div>
    </div>

    <div v-if="allCountsResult" class="derived-grid">
      <div
        v-for="(count, idx) in allCountsResult.counts"
        :key="idx"
        class="derived-cell"
        :title="`${idx}次: drag(${count.drag_x}, ${count.drag_y}) grid(${count.actual_rows}×${count.actual_cols}) overlap(${(count.overlap_x * 100).toFixed(1)}%, ${(count.overlap_y * 100).toFixed(1)}%)`"
      >
        <span class="derived-label">{{ idx }}次</span>
        <span class="derived-main">{{ count.actual_rows }}×{{ count.actual_cols }}</span>
        <span class="derived-sub">{{ (count.overlap_x * 100).toFixed(0) }}%/{{ (count.overlap_y * 100).toFixed(0) }}%</span>
        <span class="derived-drag">drag {{ count.drag_x }},{{ count.drag_y }}</span>
      </div>
    </div>
    <div v-else class="derived-hint">{{ t('data.inputTarget') }}</div>
  </div>
</template>

<script setup lang="ts">
import { defineModel } from 'vue'
import type { RegionConfig, AllCountsResult } from '@/types'
import { stripCategoryPrefix } from '@/utils/regionName'
import { useI18n } from '@/composables/useI18n'

defineProps<{
  region: RegionConfig | null
  allCountsResult: AllCountsResult | null
}>()

const targetW = defineModel<number>('targetW', { required: true })
const targetH = defineModel<number>('targetH', { required: true })

const { t } = useI18n()
</script>

<style scoped>
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

.form-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.form-label {
  font-size: 12px;
  color: var(--text-secondary);
  width: 60px;
  flex-shrink: 0;
}

.target-group {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
}

.target-input {
  flex: 1;
  min-width: 0;
}

.derived-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
  margin-top: 8px;
}

.derived-cell {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 8px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 3px;
}

.derived-label {
  font-size: 10px;
  color: var(--text-muted);
}

.derived-main {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

.derived-sub {
  font-size: 10px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.derived-drag {
  font-size: 10px;
  color: var(--accent);
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.2px;
}
</style>
