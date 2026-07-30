import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/api'
import type { AppSettings, AppSettingKey } from '@/types'
import { useAutoClearError } from '@/composables/useAutoClearError'

const DEFAULTS: AppSettings = {
  theme: 'dark',
  language: 'zh',
  output_format: 'JPG',
  jpg_quality: 95,
  original_folder: '',
  screenshot_folder: '',
  thumbnail_folder: '',
  stabilize_delay: 0.13,
  screenshot_delay: 0.13,
  drag_duration: 0.07,
  drag_margin_bottom: 10,
  drag_margin_left: 10,
  capture_offset_y: 0.02,
  overlap_min: 0.0,
  overlap_max: 0.5,
  filename_pattern: '{region}_{timestamp}_{scrollMode}',
  last_region: '武陵-武陵城',
  last_scroll_mode: '0次',
  last_aspect_ratio: '16:9',
  last_rows: 2,
  last_cols: 2,
  minimize_on_capture: true,
  accent_color: '#00b5e5',
  last_large_map_custom: false,
}

function parseSettings(all: Record<string, string>): AppSettings {
  const get = (k: AppSettingKey): string => all[k] ?? ''
  return {
    theme: (get('theme') || DEFAULTS.theme) as AppSettings['theme'],
    language: (get('language') || DEFAULTS.language) as AppSettings['language'],
    output_format: (get('output_format') || DEFAULTS.output_format) as AppSettings['output_format'],
    jpg_quality: parseInt(get('jpg_quality') || String(DEFAULTS.jpg_quality), 10),
    original_folder: get('original_folder') || DEFAULTS.original_folder,
    screenshot_folder: get('screenshot_folder') || DEFAULTS.screenshot_folder,
    thumbnail_folder: get('thumbnail_folder') || DEFAULTS.thumbnail_folder,
    stabilize_delay: parseFloat(get('stabilize_delay') || String(DEFAULTS.stabilize_delay)),
    screenshot_delay: parseFloat(get('screenshot_delay') || String(DEFAULTS.screenshot_delay)),
    drag_duration: parseFloat(get('drag_duration') || String(DEFAULTS.drag_duration)),
    drag_margin_bottom: parseInt(get('drag_margin_bottom') || String(DEFAULTS.drag_margin_bottom), 10),
    drag_margin_left: parseInt(get('drag_margin_left') || String(DEFAULTS.drag_margin_left), 10),
    capture_offset_y: parseFloat(get('capture_offset_y') || String(DEFAULTS.capture_offset_y)),
    overlap_min: parseFloat(get('overlap_min') || String(DEFAULTS.overlap_min)),
    overlap_max: parseFloat(get('overlap_max') || String(DEFAULTS.overlap_max)),
    filename_pattern: get('filename_pattern') || DEFAULTS.filename_pattern,
    last_region: get('last_region') || DEFAULTS.last_region,
    last_scroll_mode: get('last_scroll_mode') || DEFAULTS.last_scroll_mode,
    last_aspect_ratio: get('last_aspect_ratio') || DEFAULTS.last_aspect_ratio,
    last_rows: parseInt(get('last_rows') || String(DEFAULTS.last_rows), 10),
    last_cols: parseInt(get('last_cols') || String(DEFAULTS.last_cols), 10),
    minimize_on_capture: get('minimize_on_capture') !== 'false',
    accent_color: get('accent_color') || DEFAULTS.accent_color,
    last_large_map_custom: get('last_large_map_custom') === 'true',
  }
}

export const useSettingsStore = defineStore('settings', () => {
  /** 应用设置（首次 load 后非空） */
  const settings = ref<AppSettings | null>(null)
  const loading = ref(false)
  const { error: lastError, setError } = useAutoClearError()

  async function load() {
    loading.value = true
    try {
      const all = await api.getAllSettings()
      settings.value = parseSettings(all)
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      setError(msg)
      settings.value = { ...DEFAULTS }
      throw e
    } finally {
      loading.value = false
    }
  }

  async function update<K extends AppSettingKey>(key: K, value: AppSettings[K]) {
    if (!settings.value) return
    ;(settings.value as AppSettings)[key] = value
    try {
      await api.setSetting(key, String(value))
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }

  return { settings, loading, lastError, load, update }
})
