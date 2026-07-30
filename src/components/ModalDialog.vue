<script setup lang="ts">
import { state, inputEl, confirm, cancel } from '@/composables/useModal'
</script>

<template>
  <Teleport to="body">
    <div v-if="state.visible" class="md-overlay" @click.self="cancel">
      <div class="md-dialog" @click.stop>
        <h3 class="md-title">{{ state.title }}</h3>
        <p v-if="state.message" class="md-message">{{ state.message }}</p>
        <input
          v-if="state.mode === 'prompt'"
          ref="inputEl"
          v-model="state.input"
          class="md-input"
          :placeholder="state.placeholder"
          spellcheck="false"
          @keyup.enter="confirm"
          @keyup.esc="cancel"
        />
        <div class="md-actions">
          <button class="md-btn md-cancel" type="button" @click="cancel">
            {{ state.cancelText }}
          </button>
          <button
            class="md-btn md-confirm"
            :class="{ danger: state.danger }"
            type="button"
            @click="confirm"
          >
            {{ state.confirmText }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.md-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  user-select: none;
  -webkit-user-select: none;
}

.md-dialog {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 16px 18px;
  min-width: 320px;
  max-width: 460px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

.md-title {
  margin: 0 0 8px 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.md-message {
  margin: 0 0 12px 0;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.5;
}

.md-input {
  width: 100%;
  height: 28px;
  padding: 0 8px;
  border: 1px solid var(--input-border);
  border-radius: 3px;
  background: var(--input-bg);
  color: var(--text-primary);
  font-size: 12px;
  font-family: inherit;
  outline: none;
  box-sizing: border-box;
  margin-bottom: 12px;
}

.md-input:focus {
  border-color: var(--accent);
}

.md-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.md-btn {
  height: 26px;
  padding: 0 12px;
  border: 1px solid var(--border);
  border-radius: 3px;
  background: var(--bg-tertiary);
  color: var(--text-primary);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.12s ease, border-color 0.12s ease;
}

.md-btn:hover {
  background: var(--btn-hover-bg);
  border-color: var(--accent);
}

.md-confirm {
  background: var(--accent);
  border-color: var(--accent);
  color: #ffffff;
}

.md-confirm:hover {
  background: var(--accent-hover);
  border-color: var(--accent-hover);
}

.md-confirm.danger {
  background: #e81123;
  border-color: #e81123;
}

.md-confirm.danger:hover {
  background: #c50f1f;
  border-color: #c50f1f;
}
</style>
