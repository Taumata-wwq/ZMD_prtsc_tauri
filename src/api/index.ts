// Tauri Command 调用封装
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  RegionConfig,
  ScrollMode,
  CaptureSession,
  CaptureStatus,
  CaptureProgress,
  CaptureLog,
  CaptureStatusEvent,
  CropBox,
  DerivedFromBase,
  AllCountsResult,
} from '@/types'

// -----------------------------------------------------------------------------
// 配置 CRUD
// -----------------------------------------------------------------------------
export async function listRegions(): Promise<RegionConfig[]> {
  return invoke<RegionConfig[]>('list_regions')
}

export async function upsertRegion(config: RegionConfig): Promise<void> {
  return invoke<void>('upsert_region', { config })
}

export async function deleteRegion(
  name: string,
  aspectRatio: string,
  scrollMode: string,
): Promise<void> {
  return invoke<void>('delete_region', { name, aspectRatio, scrollMode })
}

export async function listScrollModes(): Promise<ScrollMode[]> {
  return invoke<ScrollMode[]>('list_scroll_modes')
}

export async function deriveAllCounts(
  clientW: number,
  clientH: number,
  targetW: number,
  targetH: number,
  overlapMin?: number,
  overlapMax?: number,
): Promise<AllCountsResult | null> {
  return invoke<AllCountsResult | null>('derive_all_counts', {
    clientW,
    clientH,
    targetW,
    targetH,
    overlapMin: overlapMin ?? 0.0,
    overlapMax: overlapMax ?? 0.5,
  })
}

export async function deriveRegionFromTarget(
  regionName: string,
  aspectRatio: string,
  scrollMode: string,
): Promise<RegionConfig | null> {
  return invoke<RegionConfig | null>('derive_region_from_target', {
    regionName,
    aspectRatio,
    scrollMode,
  })
}

// -----------------------------------------------------------------------------
// 设置
// -----------------------------------------------------------------------------
export async function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>('set_setting', { key, value })
}

export async function getAllSettings(): Promise<Record<string, string>> {
  // Rust 返回 HashMap<String, String>，TS 端等价于 Record<string, string>
  return invoke<Record<string, string>>('get_all_settings')
}

export async function setManySettings(entries: Record<string, string>): Promise<void> {
  return invoke<void>('set_many_settings', { entries })
}

// -----------------------------------------------------------------------------
// 历史
// -----------------------------------------------------------------------------
export async function listSessions(limit?: number): Promise<CaptureSession[]> {
  return invoke<CaptureSession[]>('list_sessions', { limit })
}

/** 清空所有历史记录 */
export async function clearHistory(): Promise<void> {
  return invoke<void>('clear_history')
}

// -----------------------------------------------------------------------------
// 截图
// -----------------------------------------------------------------------------
export async function startCapture(
  region: string,
  scrollMode: string,
  rows?: number,
  cols?: number,
): Promise<number> {
  // Rust 返回 session_id（i64），前端用于后续 export_image 回写
  // Tauri 默认将 Rust 端 snake_case 参数名转换为 camelCase 期望：
  //   region_name → regionName, scroll_mode → scrollMode
  return invoke<number>('start_capture', {
    regionName: region,
    scrollMode,
    rows,
    cols,
  })
}

export async function stopCapture(): Promise<void> {
  return invoke<void>('stop_capture')
}

export async function getCaptureStatus(): Promise<CaptureStatus> {
  return invoke<CaptureStatus>('get_capture_status')
}

/**
 * 拉取预览图 PNG 字节流。返回 byteLength === 0 表示暂无预览。
 */
export async function getPreviewImage(): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>('get_preview_image')
}

/**
 * 拉取预览图磁盘路径。拼接后的 PNG 已保存到磁盘，导出时直接传路径避免大数据传输。
 */
export async function getPreviewPath(): Promise<string> {
  return invoke<string>('get_preview_path')
}

/**
 * 导出图像（传 sourcePath 路径而非 data 字节流）
 */
export async function exportImage(
  sourcePath: string,
  crop: CropBox | null,
  format: string,
  quality: number,
  outputPath: string,
  sessionId?: number | null,
  cropBoxJson?: string | null,
): Promise<string> {
  return invoke<string>('export_image', {
    sourcePath,
    crop,
    format,
    quality,
    outputPath,
    sessionId,
    cropBoxJson,
  })
}

// =============================================================================
// 事件监听
// =============================================================================
/** 截图进度事件 */
export async function onCaptureProgress(cb: (e: CaptureProgress) => void): Promise<UnlistenFn> {
  return listen<CaptureProgress>('capture:progress', (event) => cb(event.payload))
}

/** 截图日志事件 */
export async function onCaptureLog(cb: (e: CaptureLog) => void): Promise<UnlistenFn> {
  return listen<CaptureLog>('capture:log', (event) => cb(event.payload))
}

/** 截图状态变更事件 */
export async function onCaptureStatus(cb: (e: CaptureStatusEvent) => void): Promise<UnlistenFn> {
  return listen<CaptureStatusEvent>('capture:status', (event) => cb(event.payload))
}

/** 预览就绪事件（携带磁盘路径，前端用 convertFileSrc 直接加载） */
export async function onCapturePreviewReady(cb: (path: string | null) => void): Promise<UnlistenFn> {
  return listen<string | null>('capture:preview-ready', (event) => cb(event.payload))
}

/** 图像处理事件（截图完成后开始拼接/编码时通知前端显示加载指示器） */
export async function onCaptureProcessing(cb: (count: number) => void): Promise<UnlistenFn> {
  return listen<number>('capture:processing', (event) => cb(event.payload))
}

// 聚合导出
export const api = {
  // 配置
  listRegions,
  upsertRegion,
  deleteRegion,
  listScrollModes,
  deriveAllCounts,
  deriveRegionFromTarget,

  // 设置
  setSetting,
  getAllSettings,
  setManySettings,

  // 历史
  listSessions,
  clearHistory,

  // 截图
  startCapture,
  stopCapture,
  getCaptureStatus,
  getPreviewImage,
  getPreviewPath,
  exportImage,
}
