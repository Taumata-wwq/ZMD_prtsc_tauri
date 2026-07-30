/** 类型定义：与 Rust models 结构体字段名一致（snake_case） */

export interface RegionConfig {
  id?: number
  name: string
  category: string  // "武陵" | "谷地" | "自定义" | "大地图"（Rust 为 String，不强制联合类型）
  aspect_ratio: string  // "16:9"
  scroll_mode: string
  grid_rows: number
  grid_cols: number
  overlap_x: number
  overlap_y: number
  drag_x: number
  drag_y: number
  capture_region_x: number
  capture_region_y: number
  capture_offset_y: number
  template_ref: string | null
  target_w: number
  target_h: number
  sub_map: string | null  // 大地图子地图名（"四号谷地"/"武陵"），仅 category="大地图" 时使用
  created_at: string
  updated_at: string
}

export interface ScrollMode {
  id?: number
  name: string
  scroll_count: number
  is_default: boolean
}

/** 基于基准数据的推导结果 */
export interface DerivedFromBase {
  img_w: number
  img_h: number
  target_w: number
  target_h: number
  actual_rows: number  // 夹紧后的实际行数（可能与 target_rows 不同）
  actual_cols: number  // 夹紧后的实际列数（可能与 target_cols 不同）
  drag_x: number
  drag_y: number
  overlap_x: number  // 0.0-1.0
  overlap_y: number  // 0.0-1.0
  clamped: boolean   // actual_rows/cols 与 target 不一致时为 true
}

/** 9 次数推导结果（索引 0-8 对应 "0次" 到 "8次"） */
export interface AllCountsResult {
  counts: DerivedFromBase[]
}

export interface CropBox {
  x: number
  y: number
  w: number
  h: number
}

export interface CaptureSession {
  id?: number
  started_at: string
  finished_at?: string | null
  region?: string | null
  scroll_mode?: string | null
  grid_rows?: number | null
  grid_cols?: number | null
  total_shots?: number | null
  status: string
  original_path?: string | null
  exported_path?: string | null
  thumbnail_path?: string | null
  crop_box?: string | null
  output_format?: string | null
  jpg_quality?: number | null
}

export interface AppSettings {
  theme: 'dark' | 'light'
  language: 'zh' | 'en'
  output_format: 'JPG' | 'PNG'
  jpg_quality: number
  original_folder: string
  screenshot_folder: string
  thumbnail_folder: string
  stabilize_delay: number
  screenshot_delay: number
  drag_duration: number
  drag_margin_bottom: number
  drag_margin_left: number
  capture_offset_y: number
  overlap_min: number
  overlap_max: number
  filename_pattern: string
  last_region: string
  last_scroll_mode: string
  last_aspect_ratio: string
  last_rows: number
  last_cols: number
  minimize_on_capture: boolean
  accent_color: string
  last_large_map_custom: boolean
}

/** 应用设置的所有键名（用于类型安全的 keyof 操作） */
export type AppSettingKey = keyof AppSettings

export interface CaptureProgress {
  current: number
  total: number
  row: number
  col: number
}

export interface CaptureLog {
  level: 'info' | 'warn' | 'error'
  message: string
  timestamp: string
}

/** capture:status 事件 payload */
export interface CaptureStatusEvent {
  is_running: boolean
  current: number
  total: number
  region: string
}
