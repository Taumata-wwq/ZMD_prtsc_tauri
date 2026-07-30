<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { clamp } from '@/utils/math'
import { toHex } from '@/utils/color'

const props = withDefaults(defineProps<{
  modelValue: string
}>(), {})

const emit = defineEmits<{
  (e: 'update:modelValue', v: string): void
}>()

interface HSB { h: number; s: number; b: number }
interface RGBA { r: number; g: number; b: number; a: number }

function parseColor(val: string): RGBA {
  if (val.startsWith('rgba') || val.startsWith('rgb')) {
    const m = val.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/)
    if (m) return { r: +m[1], g: +m[2], b: +m[3], a: m[4] !== undefined ? +m[4] : 1 }
  }
  if (val.startsWith('#')) {
    const h = val.slice(1)
    if (h.length === 3) {
      return { r: parseInt(h[0] + h[0], 16), g: parseInt(h[1] + h[1], 16), b: parseInt(h[2] + h[2], 16), a: 1 }
    }
    if (h.length === 6) {
      return { r: parseInt(h.slice(0, 2), 16), g: parseInt(h.slice(2, 4), 16), b: parseInt(h.slice(4, 6), 16), a: 1 }
    }
    if (h.length === 8) {
      return { r: parseInt(h.slice(0, 2), 16), g: parseInt(h.slice(2, 4), 16), b: parseInt(h.slice(4, 6), 16), a: parseInt(h.slice(6, 8), 16) / 255 }
    }
  }
  return { r: 0, g: 0, b: 0, a: 1 }
}

function rgbaToHsb(c: RGBA): HSB {
  const r = c.r / 255, g = c.g / 255, b = c.b / 255
  const max = Math.max(r, g, b), min = Math.min(r, g, b)
  const d = max - min
  let h = 0
  if (d !== 0) {
    if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6
    else if (max === g) h = ((b - r) / d + 2) / 6
    else h = ((r - g) / d + 4) / 6
  }
  return { h: h * 360, s: max === 0 ? 0 : d / max, b: max }
}

function hsbToRgba(hsb: HSB, a: number): RGBA {
  const h = hsb.h / 60
  const c = hsb.b * hsb.s
  const x = c * (1 - Math.abs((h % 2) - 1))
  const m = hsb.b - c
  let r = 0, g = 0, b = 0
  if (h < 1) { r = c; g = x }
  else if (h < 2) { r = x; g = c }
  else if (h < 3) { g = c; b = x }
  else if (h < 4) { g = x; b = c }
  else if (h < 5) { r = x; b = c }
  else { r = c; b = x }
  return {
    r: Math.round((r + m) * 255),
    g: Math.round((g + m) * 255),
    b: Math.round((b + m) * 255),
    a,
  }
}

function rgbaToHex(c: RGBA): string {
  if (c.a < 1) {
    return `rgba(${c.r}, ${c.g}, ${c.b}, ${Math.round(c.a * 100) / 100})`
  }
  return `#${toHex(c.r)}${toHex(c.g)}${toHex(c.b)}`
}

function rgbaToCss(c: RGBA): string {
  return `rgba(${c.r}, ${c.g}, ${c.b}, ${Math.round(c.a * 100) / 100})`
}

function formatDisplay(c: RGBA): string {
  if (c.a < 1) return rgbaToCss(c)
  return rgbaToHex(c)
}

const current = ref<RGBA>(parseColor(props.modelValue))
const hsb = ref<HSB>(rgbaToHsb(current.value))
const open = ref(false)
const rootEl = ref<HTMLElement>()
const sbCanvas = ref<HTMLElement>()
const hueSlider = ref<HTMLElement>()
const inputText = ref(formatDisplay(current.value))
const isDragging = ref(false)
const popupStyle = ref<Record<string, string>>({})

watch(() => props.modelValue, (val) => {
  const c = parseColor(val)
  current.value = c
  hsb.value = rgbaToHsb(c)
  inputText.value = formatDisplay(c)
})

const swatchStyle = computed(() => {
  return { background: rgbaToHex(current.value) }
})

const sbCanvasBg = computed(() => {
  const c = hsbToRgba({ h: hsb.value.h, s: 1, b: 1 }, 1)
  return { backgroundColor: `rgb(${c.r}, ${c.g}, ${c.b})` }
})

const sbCursorStyle = computed(() => ({
  left: `${hsb.value.s * 100}%`,
  top: `${(1 - hsb.value.b) * 100}%`,
  background: rgbaToHex(hsbToRgba({ h: hsb.value.h, s: hsb.value.s, b: hsb.value.b }, 1)),
}))

const hueCursorStyle = computed(() => ({
  left: `${(hsb.value.h / 360) * 100}%`,
  background: rgbaToHex(hsbToRgba({ h: hsb.value.h, s: 1, b: 1 }, 1)),
}))

const previewStyle = computed(() => ({ background: rgbaToCss(current.value) }))

function positionPopup() {
  if (!rootEl.value) return
  const rect = rootEl.value.getBoundingClientRect()
  const popupW = 204
  const popupH = 220
  let left = rect.left
  let top = rect.bottom + 6
  if (left + popupW > window.innerWidth - 10) left = window.innerWidth - popupW - 10
  if (left < 10) left = 10
  if (top + popupH > window.innerHeight - 10) top = rect.top - popupH - 6
  if (top < 10) top = 10
  popupStyle.value = { left: `${left}px`, top: `${top}px` }
}

function openPopup() {
  open.value = true
  nextTick(positionPopup)
}

function closePopup() { open.value = false }
function togglePopup() { open.value ? closePopup() : openPopup() }

function emitCurrentColor() {
  emit('update:modelValue', rgbaToHex(current.value))
}

function updateFromHSB() {
  const c = hsbToRgba(hsb.value, current.value.a)
  current.value = c
  inputText.value = formatDisplay(c)
}

function finishDrag() {
  isDragging.value = false
  emitCurrentColor()
}

function onInput(e: Event) {
  inputText.value = (e.target as HTMLInputElement).value
}

function onInputBlur() {
  const c = parseColor(inputText.value)
  current.value = c
  hsb.value = rgbaToHsb(c)
  inputText.value = formatDisplay(c)
  emitCurrentColor()
}

function startDragSB(e: MouseEvent | TouchEvent) {
  e.preventDefault()
  isDragging.value = true
  updateSB(e)
  const onMove = (ev: MouseEvent | TouchEvent) => { ev.preventDefault(); updateSB(ev) }
  const onUp = () => {
    finishDrag()
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('touchmove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.removeEventListener('touchend', onUp)
  }
  document.addEventListener('mousemove', onMove)
  document.addEventListener('touchmove', onMove, { passive: false })
  document.addEventListener('mouseup', onUp)
  document.addEventListener('touchend', onUp)
}

function updateSB(e: MouseEvent | TouchEvent) {
  if (!sbCanvas.value) return
  const rect = sbCanvas.value.getBoundingClientRect()
  const clientX = 'touches' in e ? e.touches[0].clientX : e.clientX
  const clientY = 'touches' in e ? e.touches[0].clientY : e.clientY
  hsb.value.s = clamp((clientX - rect.left) / rect.width, 0, 1)
  hsb.value.b = clamp(1 - (clientY - rect.top) / rect.height, 0, 1)
  updateFromHSB()
}

function startDragHue(e: MouseEvent | TouchEvent) {
  e.preventDefault()
  isDragging.value = true
  updateHue(e)
  const onMove = (ev: MouseEvent | TouchEvent) => { ev.preventDefault(); updateHue(ev) }
  const onUp = () => {
    finishDrag()
    document.removeEventListener('mousemove', onMove)
    document.removeEventListener('touchmove', onMove)
    document.removeEventListener('mouseup', onUp)
    document.removeEventListener('touchend', onUp)
  }
  document.addEventListener('mousemove', onMove)
  document.addEventListener('touchmove', onMove, { passive: false })
  document.addEventListener('mouseup', onUp)
  document.addEventListener('touchend', onUp)
}

function updateHue(e: MouseEvent | TouchEvent) {
  if (!hueSlider.value) return
  const rect = hueSlider.value.getBoundingClientRect()
  const clientX = 'touches' in e ? e.touches[0].clientX : e.clientX
  hsb.value.h = clamp(((clientX - rect.left) / rect.width) * 360, 0, 360)
  updateFromHSB()
}

function onOverlayClick() { closePopup() }

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') closePopup()
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown)
  window.addEventListener('resize', positionPopup)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown)
  window.removeEventListener('resize', positionPopup)
})
</script>

<template>
  <div class="cp-root" ref="rootEl">
    <button class="cp-swatch" @click="togglePopup" :style="swatchStyle" type="button" />
    <Teleport to="body">
      <div v-if="open" class="cp-overlay" @click="onOverlayClick" />
      <div v-if="open" class="cp-popup" :style="popupStyle" @click.stop>
        <div class="cp-sb-canvas" ref="sbCanvas" :style="sbCanvasBg" @mousedown="startDragSB">
          <div class="cp-sb-cursor" :style="sbCursorStyle" />
        </div>
        <div class="cp-sliders">
          <div class="cp-hue-slider" ref="hueSlider" @mousedown="startDragHue">
            <div class="cp-slider-cursor" :style="hueCursorStyle" />
          </div>
        </div>
        <div class="cp-input-row">
          <div class="cp-preview" :style="previewStyle" @click="togglePopup">
          </div>
          <input
            class="cp-text-input"
            :value="inputText"
            @input="onInput"
            @blur="onInputBlur"
            @keyup.enter="onInputBlur"
            spellcheck="false"
          />
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.cp-root {
  display: inline-flex;
  align-items: center;
  position: relative;
}

.cp-swatch {
  width: 15px;
  height: 15px;
  border: 1px solid var(--border);
  cursor: pointer;
  padding: 0;
  flex-shrink: 0;
  position: relative;
  overflow: hidden;
  border-radius: 2px;
  transition: border-color 0.15s;
}
.cp-swatch:hover { border-color: var(--accent); }

.cp-overlay {
  position: fixed;
  inset: 0;
  z-index: 9998;
}

.cp-popup {
  position: fixed;
  z-index: 9999;
  width: 202px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
}

.cp-sb-canvas {
  position: relative;
  width: 100%;
  height: 120px;
  border-radius: 4px;
  cursor: crosshair;
  overflow: hidden;
  margin-bottom: 8px;
}
.cp-sb-canvas::before {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(to top, #000, transparent),
              linear-gradient(to right, #fff, transparent);
  z-index: 1;
}

.cp-sb-cursor {
  position: absolute;
  z-index: 3;
  width: 14px;
  height: 14px;
  border: 2px solid #fff;
  border-radius: 50%;
  transform: translate(-50%, -50%);
  box-shadow: 0 0 2px rgba(0,0,0,0.4), inset 0 0 2px rgba(0,0,0,0.3);
  pointer-events: none;
}

.cp-sliders {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 8px;
}

.cp-hue-slider {
  position: relative;
  width: 100%;
  height: 12px;
  border-radius: 6px;
  cursor: pointer;
  background: linear-gradient(to right,
    #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00
  );
}

.cp-slider-cursor {
  position: absolute;
  top: 50%;
  z-index: 3;
  width: 16px;
  height: 16px;
  border: 2px solid #fff;
  border-radius: 50%;
  transform: translate(-50%, -50%);
  box-shadow: 0 0 2px rgba(0,0,0,0.4);
  pointer-events: none;
  background: inherit;
}

.cp-input-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.cp-preview {
  width: 28px;
  height: 28px;
  border-radius: 4px;
  border: 1px solid var(--border);
  cursor: pointer;
  flex-shrink: 0;
  position: relative;
  overflow: hidden;
}

.cp-text-input {
  flex: 1;
  min-width: 0;
  padding: 4px 8px;
  font-size: 12px;
  font-family: 'Consolas', 'Cascadia Code', monospace;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 3px;
  color: var(--text-primary);
  outline: none;
}
.cp-text-input:focus {
  border-color: var(--accent);
}
</style>
