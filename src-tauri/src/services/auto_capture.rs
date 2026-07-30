//! 自动截图服务：蛇形遍历 + SendInput 中键拖拽 + BitBlt 截图

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::IsWindow;

use crate::error::{AppError, AppResult};
use crate::models::region::RegionConfig;
use crate::models::scroll_mode::ScrollMode;
use crate::models::setting::SettingsMap;
use crate::services::game_window::WindowInfo;
use crate::services::{emit_log, game_window, input, screenshot, shared};

#[derive(serde::Serialize, Clone)]
struct ProgressPayload {
    current: u32,
    total: u32,
    row: u32,
    col: u32,
}

pub struct AutoCapture {
    is_running: Arc<AtomicBool>,
    screenshots: Mutex<Vec<(u32, u32, image::RgbaImage)>>,
    preview_png: Mutex<Option<Vec<u8>>>,
    preview_path: Mutex<Option<String>>,
    app_handle: AppHandle,
}

impl AutoCapture {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            screenshots: Mutex::new(Vec::new()),
            preview_png: Mutex::new(None),
            preview_path: Mutex::new(None),
            app_handle,
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// 消费式获取截图列表（供 stitcher 使用）
    pub async fn get_screenshots(&self) -> Vec<(u32, u32, image::RgbaImage)> {
        self.screenshots.lock().await.drain(..).collect()
    }

    /// 存储拼接后的 PNG 字节流（供前端通过 `get_preview_image` 拉取）
    pub async fn set_preview_png(&self, bytes: Vec<u8>) {
        *self.preview_png.lock().await = Some(bytes);
    }

    /// 消费式取出预览图 PNG 字节流
    pub async fn take_preview_png(&self) -> Option<Vec<u8>> {
        self.preview_png.lock().await.take()
    }

    /// 存储拼接后预览图的磁盘路径（供 export_image 直接读取，避免前端传字节流）
    pub async fn set_preview_path(&self, path: String) {
        *self.preview_path.lock().await = Some(path);
    }

    /// 获取预览图磁盘路径（非消费式，导出裁剪图和原图都需要路径）
    pub async fn get_preview_path(&self) -> Option<String> {
        self.preview_path.lock().await.clone()
    }

    /// 蛇形遍历自动截图。
    pub async fn auto_capture_grid(
        &self,
        region: &RegionConfig,
        scroll_mode: &ScrollMode,
        settings: &SettingsMap,
    ) -> AppResult<()> {
        if region.grid_rows <= 0 || region.grid_cols <= 0 {
            let err = AppError::new(
                format!("网格行列数必须大于 0（收到 {}x{}）", region.grid_rows, region.grid_cols),
                "INVALID_GRID",
            );
            emit_log(&self.app_handle, "error", err.message.clone());
            return Err(err);
        }

        let total = (region.grid_rows as u32) * (region.grid_cols as u32);

        self.is_running.store(true, Ordering::SeqCst);
        self.screenshots.lock().await.clear();

        emit_log(&self.app_handle, "info", "开始自动截图...");
        shared::emit_status(&self.app_handle, true, 0, total, &region.name);

        // 截图前最小化主窗口，避免它抢焦点导致鼠标拖拽事件被 webview 接收而非到达游戏
        let minimize_on_capture = settings
            .get("minimize_on_capture")
            .map(|v| v != "false")
            .unwrap_or(true);
        let main_window = self.app_handle.get_webview_window("main");
        let was_maximized = main_window.as_ref().and_then(|w| w.is_maximized().ok()).unwrap_or(false);
        if minimize_on_capture {
            if let Some(window) = &main_window {
                let _ = window.minimize();
                emit_log(&self.app_handle, "info", "已最小化主窗口，让出焦点给游戏");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }

        let app_handle = self.app_handle.clone();
        let is_running = self.is_running.clone();
        let region_c = region.clone();
        let scroll_mode_c = scroll_mode.clone();
        let settings_c = settings.clone();

        let join_result = tokio::task::spawn_blocking(
            move || -> AppResult<Vec<(u32, u32, image::RgbaImage)>> {
                capture_grid_blocking(
                    &app_handle, &is_running, &region_c, &scroll_mode_c, &settings_c,
                )
            },
        )
        .await;

        self.is_running.store(false, Ordering::SeqCst);

        if minimize_on_capture {
            if let Some(window) = &main_window {
                let _ = window.unminimize();
                if was_maximized {
                    let _ = window.maximize();
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        match join_result {
            Ok(Ok(shots)) => {
                let mut guard = self.screenshots.lock().await;
                let count = shots.len();
                *guard = shots;
                emit_log(&self.app_handle, "info", format!("完成 {} 张截图", count));
                // 后续还有拼接+编码流程，由 capture:preview-ready 触发状态重置
                Ok(())
            }
            Ok(Err(e)) => {
                emit_log(&self.app_handle, "error", format!("截图失败: {} ({})", e.message, e.code));
                shared::emit_status(&self.app_handle, false, 0, total, &region.name);
                Err(e)
            }
            Err(e) => {
                let err = AppError::new(format!("截图任务执行失败: {}", e), "TASK_JOIN_ERROR");
                emit_log(&self.app_handle, "error", err.message.clone());
                shared::emit_status(&self.app_handle, false, 0, total, &region.name);
                Err(err)
            }
        }
    }
}

// 同步核心流程

/// 截图配置：由 `read_capture_settings` 从 settings + region 读取，避免子函数重复读取。
struct CaptureSettings {
    stabilize_delay: f64,
    screenshot_delay: f64,
    drag_duration: f64,
    drag_margin_bottom: i32,
    drag_margin_left: i32,
    capture_region_x: f64,
    capture_region_y: f64,
    capture_offset_y: f64,
}

/// 准备截图阶段的中间结果
struct PrepareResult {
    window_info: WindowInfo,
    settings: CaptureSettings,
}

/// 同步执行完整截图流程：准备 → 检查运行状态 → 蛇形遍历
fn capture_grid_blocking(
    app_handle: &AppHandle,
    is_running: &Arc<AtomicBool>,
    region: &RegionConfig,
    scroll_mode: &ScrollMode,
    settings: &SettingsMap,
) -> AppResult<Vec<(u32, u32, image::RgbaImage)>> {
    let prep = match validate_and_prepare_capture(app_handle, is_running, region, scroll_mode, settings)? {
        Some(p) => p,
        None => {
            emit_log(app_handle, "warn", "截图已被用户停止");
            return Ok(Vec::new());
        }
    };
    run_serpentine_loop(app_handle, is_running, region, &prep)
}

/// 准备截图：查找窗口、激活、读取设置、执行滚轮。返回 `None` 表示已被用户停止。
fn validate_and_prepare_capture(
    app_handle: &AppHandle,
    is_running: &Arc<AtomicBool>,
    region: &RegionConfig,
    scroll_mode: &ScrollMode,
    settings: &SettingsMap,
) -> AppResult<Option<PrepareResult>> {
    let window_info = match find_and_activate_window(app_handle, is_running)? {
        Some(w) => w,
        None => return Ok(None),
    };
    let capture_settings = read_capture_settings(settings, region);
    match do_scroll_blocking(
        app_handle, is_running, scroll_mode.scroll_count, window_info.hwnd,
        window_info.left, window_info.top,
        capture_settings.drag_margin_bottom, capture_settings.drag_margin_left,
    )? {
        Some(()) => {}
        None => return Ok(None),
    }
    Ok(Some(PrepareResult { window_info, settings: capture_settings }))
}

/// 查找并激活窗口。返回 `None` 表示已被用户停止。
/// activate_window 内部使用 CACHED_HWND，需先 find_endfield_window 填充缓存。
fn find_and_activate_window(
    app_handle: &AppHandle,
    is_running: &Arc<AtomicBool>,
) -> AppResult<Option<WindowInfo>> {
    let window_info = game_window::find_endfield_window()
        .map_err(|e| map_err_with_log(app_handle, e, "WINDOW_FIND_FAILED", "查找窗口失败"))?
        .ok_or_else(|| AppError::new("未找到终末地窗口，请先打开游戏", "WINDOW_NOT_FOUND"))?;
    emit_log(app_handle, "info", format!("找到窗口：{}", window_info.title));
    game_window::activate_window()
        .map_err(|e| map_err_with_log(app_handle, e, "WINDOW_ACTIVATE_FAILED", "激活窗口失败"))?;
    if !is_running.load(Ordering::SeqCst) {
        emit_log(app_handle, "warn", "截图已被用户停止");
        return Ok(None);
    }
    Ok(Some(window_info))
}

/// 读取截图配置（带默认值兜底）。
/// capture_region_x/y 从 region 配置读取而非全局 settings：不同缩放级别下视图移动倍率不同，drag≠step。
fn read_capture_settings(settings: &SettingsMap, region: &RegionConfig) -> CaptureSettings {
    CaptureSettings {
        stabilize_delay: get_f64(settings, "stabilize_delay", 0.08),
        screenshot_delay: get_f64(settings, "screenshot_delay", 0.01),
        drag_duration: get_f64(settings, "drag_duration", 0.01),
        drag_margin_bottom: settings.get("drag_margin_bottom")
            .and_then(|v| v.parse::<i32>().ok()).unwrap_or(10),
        drag_margin_left: settings.get("drag_margin_left")
            .and_then(|v| v.parse::<i32>().ok()).unwrap_or(10),
        capture_region_x: region.capture_region_x,
        capture_region_y: region.capture_region_y,
        capture_offset_y: get_f64(settings, "capture_offset_y", 0.02),
    }
}

/// 执行滚轮 + 拖拽刷新。scroll_count <= 0 时跳过滚轮。返回 `None` 表示已被用户停止。
#[allow(clippy::too_many_arguments)]
fn do_scroll_blocking(
    app_handle: &AppHandle,
    is_running: &Arc<AtomicBool>,
    scroll_count: i32,
    hwnd: isize,
    window_left: i32,
    window_top: i32,
    margin_bottom: i32,
    margin_left: i32,
) -> AppResult<Option<()>> {
    if scroll_count <= 0 {
        emit_log(app_handle, "info", "跳过滚轮滚动（scroll_count=0）");
        return Ok(Some(()));
    }
    emit_log(app_handle, "info", format!("执行滚轮滚动：向下滚动 {} 次", scroll_count));

    let client_rect = game_window::get_client_rect(hwnd)
        .map_err(|e| map_err_with_log(app_handle, e, "GET_CLIENT_RECT_FAILED", "获取客户区失败"))?;
    let center_x = client_rect.left + client_rect.width / 2;
    let center_y = client_rect.top + client_rect.height / 2;

    let is_running_clone = is_running.clone();
    input::do_scroll(scroll_count, center_x, center_y, || is_running_clone.load(Ordering::SeqCst));

    if !is_running.load(Ordering::SeqCst) {
        return Ok(None);
    }

    emit_log(app_handle, "info", "拖拽画面刷新...");
    input::refresh_view(window_left, window_top, margin_bottom, margin_left);
    emit_log(app_handle, "info", "滚轮滚动完成");
    Ok(Some(()))
}

/// 执行蛇形遍历截图。
fn run_serpentine_loop(
    app_handle: &AppHandle,
    is_running: &Arc<AtomicBool>,
    region: &RegionConfig,
    prep: &PrepareResult,
) -> AppResult<Vec<(u32, u32, image::RgbaImage)>> {
    let rows = region.grid_rows as u32;
    let cols = region.grid_cols as u32;
    let total = rows * cols;
    let drag_distance = (region.drag_x, region.drag_y);
    let mut screenshots: Vec<(u32, u32, image::RgbaImage)> = Vec::with_capacity(total as usize);
    let mut current: u32 = 0;

    emit_log(app_handle, "info",
        format!("开始蛇形遍历：{} 行 {} 列，共 {} 张", rows, cols, total));

    for row in 0..rows {
        if !is_running.load(Ordering::SeqCst) {
            break;
        }
        // 蛇形：偶数行左→右，奇数行右→左
        let is_even = row % 2 == 0;
        let col_indices: Vec<u32> = if is_even {
            (0..cols).collect()
        } else {
            (0..cols).rev().collect()
        };

        for (idx, &col) in col_indices.iter().enumerate() {
            if !is_running.load(Ordering::SeqCst) {
                break;
            }
            current = capture_one_cell(app_handle, prep, row, col, current, total, &mut screenshots)?;
            // 行内移动（非最后一张）；move_camera 内部末尾已含 stabilize_delay
            if idx < col_indices.len() - 1 {
                move_camera_in_row(app_handle, prep, is_even, drag_distance);
            }
        }

        if row < rows - 1 && is_running.load(Ordering::SeqCst) {
            emit_log(app_handle, "info",
                format!("下移：距离=({}, {}) 时长={:.3}s",
                    drag_distance.0, drag_distance.1, prep.settings.drag_duration));
            move_camera_blocking(
                input::CameraDirection::Down, drag_distance, prep.window_info.hwnd,
                prep.settings.drag_margin_bottom, prep.settings.drag_margin_left,
                prep.settings.drag_duration, prep.settings.stabilize_delay,
            );
        }
    }

    Ok(screenshots)
}

/// 截取当前格子：延迟 → 验证窗口 → 截图 → push → emit 进度。返回更新后的 current。
fn capture_one_cell(
    app_handle: &AppHandle,
    prep: &PrepareResult,
    row: u32,
    col: u32,
    current: u32,
    total: u32,
    screenshots: &mut Vec<(u32, u32, image::RgbaImage)>,
) -> AppResult<u32> {
    std::thread::sleep(input::rand_delay(prep.settings.screenshot_delay));

    if !is_window_valid(prep.window_info.hwnd) {
        emit_log(app_handle, "error", "游戏窗口已关闭");
        return Err(AppError::new("游戏窗口已关闭", "WINDOW_CLOSED"));
    }

    let img = match screenshot::capture_center_region(
        prep.window_info.hwnd,
        prep.settings.capture_region_x,
        prep.settings.capture_region_y,
        prep.settings.capture_offset_y,
    ) {
        Ok(img) => img,
        Err(e) => {
            emit_log(app_handle, "error",
                format!("截图失败 (行{}列{}): {}", row + 1, col + 1, e.message));
            return Err(e);
        }
    };

    screenshots.push((row, col, img));
    let new_current = current + 1;

    emit_progress(app_handle, new_current, total, row, col);
    emit_log(app_handle, "info",
        format!("已截图 {}/{} (行{}列{})", new_current, total, row + 1, col + 1));

    Ok(new_current)
}

/// 行内移动相机到下一个格子（拖拽）。
fn move_camera_in_row(
    app_handle: &AppHandle,
    prep: &PrepareResult,
    is_even: bool,
    drag_distance: (i32, i32),
) {
    let direction = if is_even {
        input::CameraDirection::Right
    } else {
        input::CameraDirection::Left
    };
    emit_log(app_handle, "info",
        format!("拖拽方向={:?} 距离=({}, {}) 时长={:.3}s",
            direction, drag_distance.0, drag_distance.1, prep.settings.drag_duration));
    move_camera_blocking(
        direction, drag_distance, prep.window_info.hwnd,
        prep.settings.drag_margin_bottom, prep.settings.drag_margin_left,
        prep.settings.drag_duration, prep.settings.stabilize_delay,
    );
}

/// 同步执行相机移动（SendInput + 中键拖拽）
fn move_camera_blocking(
    direction: input::CameraDirection,
    drag_distance: (i32, i32),
    hwnd: isize,
    margin_bottom: i32,
    margin_left: i32,
    drag_duration: f64,
    stabilize_delay: f64,
) {
    let client_rect = match game_window::get_client_rect(hwnd) {
        Ok(r) => r,
        Err(_) => return,
    };
    input::move_camera(
        direction, drag_distance, client_rect.left, client_rect.top, client_rect.height,
        margin_bottom, margin_left, drag_duration, stabilize_delay,
    );
}

// 辅助函数

/// 构造带日志的错误：包装为 `AppError` 并 emit error 日志
fn map_err_with_log<E: std::fmt::Display>(
    app: &AppHandle,
    e: E,
    code: &str,
    msg: &str,
) -> AppError {
    let error_msg = format!("{}: {}", msg, e);
    emit_log(app, "error", error_msg.clone());
    AppError::new(error_msg, code)
}

fn is_window_valid(hwnd_raw: isize) -> bool {
    let hwnd = HWND(hwnd_raw as *mut _);
    unsafe { IsWindow(Some(hwnd)) }.as_bool()
}

/// 从 `SettingsMap` 读取 f64（解析失败返回 default）
fn get_f64(settings: &SettingsMap, key: &str, default: f64) -> f64 {
    settings.get(key).and_then(|v| v.parse::<f64>().ok()).unwrap_or(default)
}

/// emit 进度事件
fn emit_progress(app_handle: &AppHandle, current: u32, total: u32, row: u32, col: u32) {
    let payload = ProgressPayload { current, total, row, col };
    let _ = app_handle.emit("capture:progress", payload);
}
