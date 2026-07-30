<template>
  <div class="edit-content">
    <header class="edit-header">
      <h3 class="edit-title">{{ subMapName }}</h3>
      <span class="edit-subtitle">{{ t('data.tabLargeMap') }}</span>
    </header>

    <div v-if="areas.length === 0" class="derived-hint">
      {{ t('data.empty') }}
    </div>

    <div v-else class="lm-area-list">
      <div
        v-for="(area, idx) in areas"
        :key="area.region.name + idx"
        class="lm-area-row"
      >
        <span class="lm-area-name">{{ area.name }}</span>
        <input
          v-model.number="area.region.grid_rows"
          type="number"
          class="form-input lm-grid-input"
          min="1"
          max="100"
          @change="emit('update:area', area)"
        />
        <span class="target-sep">×</span>
        <input
          v-model.number="area.region.grid_cols"
          type="number"
          class="form-input lm-grid-input"
          min="1"
          max="100"
          @change="emit('update:area', area)"
        />
        <button
          class="btn btn-danger btn-sm"
          type="button"
          @click="emit('delete:area', area)"
        >{{ t('data.delete') }}</button>
      </div>
    </div>

    <button
      class="btn btn-primary btn-sm lm-add-btn"
      type="button"
      @click="emit('add:area')"
    >+ {{ t('data.addArea') }}</button>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from '@/composables/useI18n'
import type { LargeMapArea } from '@/composables/useRegionTree'

defineProps<{
  subMapName: string
  areas: LargeMapArea[]
}>()

const emit = defineEmits<{
  (e: 'update:area', area: LargeMapArea): void
  (e: 'delete:area', area: LargeMapArea): void
  (e: 'add:area'): void
}>()

const { t } = useI18n()
</script>

<style scoped>
.lm-area-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.lm-area-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.lm-area-name {
  flex: 2;
  min-width: 0;
  font-size: 12px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lm-grid-input {
  flex: 1;
  min-width: 0;
  text-align: center;
}

.lm-add-btn {
  align-self: flex-start;
  margin-top: 8px;
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 4px 10px;
  border: 1px solid var(--border);
  border-radius: 3px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.12s ease;
}

.btn:hover {
  background: var(--bg-tertiary);
}

.btn-sm {
  padding: 3px 8px;
  font-size: 11px;
}

.btn-primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.btn-primary:hover {
  background: var(--accent-hover);
  border-color: var(--accent-hover);
}

.btn-danger {
  color: #e81123;
  border-color: var(--border);
}

.btn-danger:hover {
  background: #e81123;
  color: #fff;
  border-color: #e81123;
}
</style>
