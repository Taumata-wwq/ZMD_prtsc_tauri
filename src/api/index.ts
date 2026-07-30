/** Tauri Command 调用封装 */
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  RegionConfig,
  ScrollMode,
  CaptureSession,
  CaptureProgress,
  CaptureLog,
  CaptureStatusEvent,
  CropBox,
  AllCountsResult,
} from '@/types'

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

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>('set_setting', { key, value })
}

export async function getAllSettings(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>('get_all_settings')
}

export async function resetData(includeHistory: boolean): Promise<void> {
  return invoke<void>('reset_data', { includeHistory })
}

export async function listSessions(): Promise<CaptureSession[]> {
  return invoke<CaptureSession[]>('list_sessions')
}

export async function deleteSession(
  sessionId: number,
  deleteOriginal: boolean,
  deleteScreenshot: boolean,
): Promise<void> {
  return invoke<void>('delete_session', {
    sessionId,
    deleteOriginal,
    deleteScreenshot,
  })
}

export async function startCapture(
  region: string,
  scrollMode: string,
  rows?: number,
  cols?: number,
): Promise<number> {
  // Rust 返回 session_id（i64），前端用于 export_image 回写
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

/** 拉取预览图 PNG 字节流。返回 byteLength === 0 表示暂无预览 */
export async function getPreviewImage(): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>('get_preview_image')
}

/** 拉取预览图磁盘路径（拼接后已保存到磁盘，导出时直接传路径避免大数据传输） */
export async function getPreviewPath(): Promise<string> {
  return invoke<string>('get_preview_path')
}

/** 设置预览图磁盘路径（用于从历史记录加载原图重新编辑） */
export async function setPreviewPath(path: string): Promise<void> {
  return invoke<void>('set_preview_path', { path })
}

/** 导出图像（传 sourcePath 路径而非 data 字节流） */
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

export async function onCaptureProgress(cb: (e: CaptureProgress) => void): Promise<UnlistenFn> {
  return listen<CaptureProgress>('capture:progress', (event) => cb(event.payload))
}

export async function onCaptureLog(cb: (e: CaptureLog) => void): Promise<UnlistenFn> {
  return listen<CaptureLog>('capture:log', (event) => cb(event.payload))
}

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

export const api = {
  listRegions,
  upsertRegion,
  deleteRegion,
  listScrollModes,
  deriveAllCounts,
  deriveRegionFromTarget,
  setSetting,
  getAllSettings,
  resetData,
  listSessions,
  deleteSession,
  startCapture,
  stopCapture,
  getPreviewImage,
  getPreviewPath,
  setPreviewPath,
  exportImage,
}
