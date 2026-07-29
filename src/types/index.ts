// 类型定义：与 Rust models 结构体字段名一致（snake_case）

// 区域配置（对应 Rust RegionConfig）
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
  created_at: string
  updated_at: string
}

// 滚动模式（对应 Rust ScrollMode）
export interface ScrollMode {
  id?: number
  name: string
  scroll_count: number
  is_default: boolean
}

// 基于基准数据的推导结果（对应 Rust DerivedFromBaseDto）
export interface DerivedFromBase {
  /** 单张截图宽（像素） */
  img_w: number
  /** 单张截图高（像素） */
  img_h: number
  /** 反推得到的目标总宽（像素，恒定） */
  target_w: number
  /** 反推得到的目标总高（像素，恒定） */
  target_h: number
  /** 夹紧后的实际行数（可能与 target_rows 不同） */
  actual_rows: number
  /** 夹紧后的实际列数（可能与 target_cols 不同） */
  actual_cols: number
  /** 最终 drag_x（像素） */
  drag_x: number
  /** 最终 drag_y（像素） */
  drag_y: number
  /** 最终 overlap_x（0.0-1.0） */
  overlap_x: number
  /** 最终 overlap_y（0.0-1.0） */
  overlap_y: number
  /** 是否发生了夹紧（actual_rows/cols 与 target 不一致） */
  clamped: boolean
}

// 9 次数推导结果（对应 Rust AllCountsResultDto）
// 索引 0-8 对应 "0次" 到 "8次"
export interface AllCountsResult {
  counts: DerivedFromBase[]
}

// 截图会话（对应 Rust CaptureSession）
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
  status: string  // SessionStatus 字符串值；Rust 端存储为 String
  original_path?: string | null
  exported_path?: string | null
  crop_box?: string | null  // JSON 字符串：JSON.stringify(CropBox)；与 Rust Option<String> 对齐
  output_format?: string | null
  jpg_quality?: number | null
}

// 应用设置（对应 Rust app_setting 表）
export interface AppSettings {
  theme: 'dark' | 'light'
  language: 'zh' | 'en'  // UI 语言（中英文切换）
  output_format: 'JPG' | 'PNG'  // Rust 默认值 "JPG"（大写）
  jpg_quality: number  // 1-100
  output_folder: string
  stabilize_delay: number  // 秒（浮点），默认 0.1
  screenshot_delay: number  // 秒（浮点），默认 0.1
  drag_duration: number  // 秒（浮点），默认 0.05
  drag_margin_bottom: number
  drag_margin_left: number
  capture_offset_y: number
  // overlap 硬约束范围（用于 derive_from_base 夹紧行列数）
  overlap_min: number  // 比例，默认 0.0（允许 0% 重叠）
  overlap_max: number  // 比例，默认 0.5（最大 50% 重叠）
  // 自定义导出文件名格式
  filename_pattern: string
  last_region: string
  last_scroll_mode: string
  last_aspect_ratio: string
  last_rows: number
  last_cols: number
  // 开始截图后是否最小化窗口
  minimize_on_capture: boolean
}

// 应用设置的所有键名（用于类型安全的 keyof 操作）
export type AppSettingKey = keyof AppSettings

// 截图事件与状态
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

export interface CaptureStatus {
  is_running: boolean
  current: number
  total: number
  region?: string
}

// capture:status 事件 payload
export interface CaptureStatusEvent {
  is_running: boolean
  current: number
  total: number
  region: string
}
