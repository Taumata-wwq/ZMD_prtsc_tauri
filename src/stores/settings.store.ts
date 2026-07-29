// 应用设置 Store：Rust 端 key-value 存储的前端强类型视图
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/api'
import type { AppSettings, AppSettingKey } from '@/types'
import { useAutoClearError } from '@/composables/useAutoClearError'

/**
 * 数值型字段集合（用于 string ↔ number 转换判断）
 *
 * 注意：Rust 端这些字段在 DB 中均以 String 存储（"0.08" / "2" / "95"），
 * 但前端语义上为 number。整数与浮点不区分，统一用 parseFloat 解析。
 */
const NUMERIC_KEYS: ReadonlySet<AppSettingKey> = new Set<AppSettingKey>([
  'jpg_quality',
  'stabilize_delay',
  'screenshot_delay',
  'drag_duration',
  'drag_margin_bottom',
  'drag_margin_left',
  'capture_offset_y',
  'overlap_min',
  'overlap_max',
  'last_rows',
  'last_cols',
])

/** 默认值（与 Rust default_settings() 保持一致） */
const DEFAULTS: AppSettings = {
  theme: 'dark',
  language: 'zh',  // UI 语言，默认中文
  output_format: 'JPG',  // Rust 默认 "JPG"（大写）
  jpg_quality: 95,
  output_folder: '',  // 空=使用 app_data_dir/screenshots（由后端处理）
  stabilize_delay: 0.1,    // 100ms（范围 30-500ms）
  screenshot_delay: 0.1,   // 100ms（范围 30-500ms）
  drag_duration: 0.05,    // 50ms（范围 30-500ms）
  drag_margin_bottom: 10,
  drag_margin_left: 10,
  capture_offset_y: 0.02,
  // overlap 硬约束范围（用于 derive_from_base 夹紧行列数）
  overlap_min: 0.0,  // 允许 0% 重叠
  overlap_max: 0.5,  // 最大 50% 重叠
  filename_pattern: '{region}_{timestamp}_{scrollMode}',
  last_region: '武陵-武陵城',
  last_scroll_mode: '0次',
  last_aspect_ratio: '16:9',
  last_rows: 2,
  last_cols: 2,
  minimize_on_capture: true,
}

/**
 * 将 Rust 返回的 Record<string, string> 转换为前端强类型 AppSettings
 * 缺失的键使用 DEFAULTS 兜底。
 */
function parseSettings(all: Record<string, string>): AppSettings {
  const get = (k: AppSettingKey): string => all[k] ?? ''
  return {
    theme: (get('theme') || DEFAULTS.theme) as AppSettings['theme'],
    language: (get('language') || DEFAULTS.language) as AppSettings['language'],
    output_format: (get('output_format') || DEFAULTS.output_format) as AppSettings['output_format'],
    jpg_quality: parseInt(get('jpg_quality') || String(DEFAULTS.jpg_quality), 10),
    output_folder: get('output_folder') || DEFAULTS.output_folder,
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
  }
}

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  /** 最近一次错误信息（null 表示无错误，3 秒后自动清空，供全局错误条展示） */
  const { error: lastError, setError } = useAutoClearError()

  /** 从后端加载全部设置 */
  async function load() {
    loading.value = true
    error.value = null
    try {
      const all = await api.getAllSettings()
      settings.value = parseSettings(all)
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      error.value = msg
      setError(msg)
      // 失败时回退到默认值，保证 UI 可用
      settings.value = { ...DEFAULTS }
      throw e
    } finally {
      loading.value = false
    }
  }

  /**
   * 更新单个设置项
   *
   * @param key 设置键名
   * @param value 新值（string | number；number 会自动转 string 写入后端）
   */
  async function update(key: AppSettingKey, value: string | number) {
    if (!settings.value) return
    const strValue = String(value)
    // 本地立即更新（数值字段转 number，字符串字段保持）
    if (NUMERIC_KEYS.has(key)) {
      ;(settings.value as any)[key] = typeof value === 'number' ? value : parseFloat(value)
    } else {
      ;(settings.value as any)[key] = value
    }
    // 后端同步
    try {
      await api.setSetting(key, strValue)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      throw err
    }
  }

  return { settings, loading, error, lastError, load, update }
})
