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
use crate::services::{emit_log, game_window, input, screenshot};

// 事件 payload

/// 进度事件（`capture:progress`）
#[derive(serde::Serialize, Clone)]
struct ProgressPayload {
    current: u32,
    total: u32,
    row: u32,
    col: u32,
}

/// 状态事件（`capture:status`）
#[derive(serde::Serialize, Clone)]
struct StatusPayload {
    is_running: bool,
    current: u32,
    total: u32,
    region: String,
}

// AutoCapture

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

    /// 是否正在运行
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// 停止截图（设置标志位，主循环会在下次检查时退出）
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// 获取截图列表（消费式，供 stitcher 使用）
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

    /// 获取预览图磁盘路径（非消费式，可多次调用）
    ///
    /// 导出裁剪图和原图都需要路径，使用 clone 返回，路径在 AutoCapture 生命周期内始终可用。
    pub async fn get_preview_path(&self) -> Option<String> {
        self.preview_path.lock().await.clone()
    }

    /// 蛇形遍历自动截图，复刻原 Python `auto_capture_grid` 逻辑。
    pub async fn auto_capture_grid(
        &self,
        region: &RegionConfig,
        scroll_mode: &ScrollMode,
        settings: &SettingsMap,
    ) -> AppResult<()> {
        // 1. 校验网格参数
        if region.grid_rows <= 0 || region.grid_cols <= 0 {
            let err = AppError::new(
                format!(
                    "网格行列数必须大于 0（收到 {}x{}）",
                    region.grid_rows, region.grid_cols
                ),
                "INVALID_GRID",
            );
            emit_log(&self.app_handle, "error", err.message.clone());
            return Err(err);
        }

        let total = (region.grid_rows as u32) * (region.grid_cols as u32);

        // 2. 设置运行标志 + 清空旧截图
        self.is_running.store(true, Ordering::SeqCst);
        self.screenshots.lock().await.clear();

        // 3. emit 开始状态
        emit_log(&self.app_handle, "info", "开始自动截图...");
        emit_status(&self.app_handle, true, 0, total, &region.name);

        // 根据 minimize_on_capture 设置决定是否最小化主窗口
        // 截图开始前最小化 Tauri 主窗口，避免它抢焦点导致
        // 鼠标拖拽事件被 webview 接收而非到达游戏。
        // 调用失败不阻止截图，仅打印警告。
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
                // 等待 200ms 让 OS 完成最小化动画
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }

        // 4. clone 必要数据（spawn_blocking 闭包需要 'static）
        let app_handle = self.app_handle.clone();
        let is_running = self.is_running.clone();
        let region_c = region.clone();
        let scroll_mode_c = scroll_mode.clone();
        let settings_c = settings.clone();

        // 5. 在阻塞线程中执行同步工作
        //    原因：move_camera/do_scroll/refresh_view 均为同步函数，
        //    内部使用 thread::sleep（几十~几百毫秒），直接在 async 上下文调用会阻塞 tokio。
        let join_result = tokio::task::spawn_blocking(
            move || -> AppResult<Vec<(u32, u32, image::RgbaImage)>> {
                capture_grid_blocking(
                    &app_handle,
                    &is_running,
                    &region_c,
                    &scroll_mode_c,
                    &settings_c,
                )
            },
        )
        .await;

        // 6. 重置运行标志
        self.is_running.store(false, Ordering::SeqCst);

        // 7. 恢复主窗口（截图完成后恢复显示；仅在被最小化时恢复）
        if minimize_on_capture {
            if let Some(window) = &main_window {
                let _ = window.unminimize();
                // 如果之前是最大化状态，恢复最大化
                if was_maximized {
                    let _ = window.maximize();
                }
                // 短暂延迟让 OS 完成恢复
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        // 8. 处理结果
        match join_result {
            Ok(Ok(shots)) => {
                let mut guard = self.screenshots.lock().await;
                let count = shots.len();
                *guard = shots;
                emit_log(
                    &self.app_handle,
                    "info",
                    format!("完成 {} 张截图", count),
                );
                // 成功路径不在此处 emit capture:status(is_running=false)
                // 因为后续还有拼接 + 编码流程（由 capture.rs spawn 任务执行）。
                // 若过早 emit is_running=false，前端会进入"等待截图"空窗期。
                // is_running 状态由前端在 capture:preview-ready 事件处理中重置。
                Ok(())
            }
            Ok(Err(e)) => {
                emit_log(
                    &self.app_handle,
                    "error",
                    format!("截图失败: {} ({})", e.message, e.code),
                );
                emit_status(&self.app_handle, false, 0, total, &region.name);
                Err(e)
            }
            Err(e) => {
                let err = AppError::new(
                    format!("截图任务执行失败: {}", e),
                    "TASK_JOIN_ERROR",
                );
                emit_log(&self.app_handle, "error", err.message.clone());
                emit_status(&self.app_handle, false, 0, total, &region.name);
                Err(err)
            }
        }
    }

    /// 执行滚轮 + 拖拽刷新
    ///
    /// scroll_count <= 0 时跳过滚轮，直接返回。
    fn do_scroll_blocking(
        app_handle: &AppHandle,
        is_running: &Arc<AtomicBool>,
        scroll_count: i32,
        hwnd: isize,
        window_left: i32,
        window_top: i32,
        margin_bottom: i32,
        margin_left: i32,
    ) -> AppResult<()> {
        if scroll_count <= 0 {
            emit_log(app_handle, "info", "跳过滚轮滚动（scroll_count=0）");
            return Ok(());
        }

        emit_log(
            app_handle,
            "info",
            format!("执行滚轮滚动：向下滚动 {} 次", scroll_count),
        );

        // 获取客户区以计算滚轮中心点（对应原 Python center_x/center_y）
        let client_rect = game_window::get_client_rect(hwnd).map_err(|e| {
            let err = AppError::new(format!("获取客户区失败: {}", e), "GET_CLIENT_RECT_FAILED");
            emit_log(app_handle, "error", err.message.clone());
            err
        })?;
        let center_x = client_rect.left + client_rect.width / 2;
        let center_y = client_rect.top + client_rect.height / 2;

        // 执行滚轮（input::do_scroll 内部循环 scroll_count 次，每次后 sleep 150ms，末尾 sleep 500ms）
        let is_running_clone = is_running.clone();
        input::do_scroll(
            scroll_count,
            center_x,
            center_y,
            || is_running_clone.load(Ordering::SeqCst),
        );

        if !is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        // 拖拽刷新画面（对应原 Python do_scroll 末尾的拖拽逻辑）
        // 移除多余的 stabilize_delay，与 Python 原版对齐
        // input::do_scroll 末尾已有 500ms 等待，refresh_view 末尾已有 50ms 等待
        emit_log(app_handle, "info", "拖拽画面刷新...");
        input::refresh_view(window_left, window_top, margin_bottom, margin_left);

        emit_log(app_handle, "info", "滚轮滚动完成");
        Ok(())
    }
}

// 同步核心流程（在 spawn_blocking 中调用）

/// 同步执行完整截图流程，返回截图列表。
fn capture_grid_blocking(
    app_handle: &AppHandle,
    is_running: &Arc<AtomicBool>,
    region: &RegionConfig,
    scroll_mode: &ScrollMode,
    settings: &SettingsMap,
) -> AppResult<Vec<(u32, u32, image::RgbaImage)>> {
    // 1. 查找游戏窗口
    let window_info = game_window::find_endfield_window()
        .map_err(|e| {
            let err = AppError::new(format!("查找窗口失败: {}", e), "WINDOW_FIND_FAILED");
            emit_log(app_handle, "error", err.message.clone());
            err
        })?
        .ok_or_else(|| {
            let err = AppError::new("未找到终末地窗口，请先打开游戏", "WINDOW_NOT_FOUND");
            emit_log(app_handle, "error", err.message.clone());
            err
        })?;

    emit_log(
        app_handle,
        "info",
        format!("找到窗口：{}", window_info.title),
    );

    // 2. 激活窗口（activate_window 内部使用 CACHED_HWND，
    //    需先 find_endfield_window 填充缓存）— 不置顶
    game_window::activate_window().map_err(|e| {
        let err = AppError::new(format!("激活窗口失败: {}", e), "WINDOW_ACTIVATE_FAILED");
        emit_log(app_handle, "error", err.message.clone());
        err
    })?;

    if !is_running.load(Ordering::SeqCst) {
        emit_log(app_handle, "warn", "截图已被用户停止");
        return Ok(Vec::new());
    }

    // 3. 读取设置（带默认值兜底）
    let stabilize_delay = get_f64(settings, "stabilize_delay", 0.08);
    let screenshot_delay = get_f64(settings, "screenshot_delay", 0.01);
    let drag_duration = get_f64(settings, "drag_duration", 0.01);
    // 拖拽时鼠标距离边界的距离（从设置读取，默认 10px）
    let drag_margin_bottom = settings
        .get("drag_margin_bottom")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(10);
    let drag_margin_left = settings
        .get("drag_margin_left")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(10);
    // capture_region_x/y 从 region 配置读取，而非全局 settings
    // region.capture_region_x/y 统一为 0.626/0.648（大地图除外：0.378/0.388）
    // 游戏在不同缩放级别下有不同视图移动倍率，drag≠step，
    // Python 的 overlap 值是基于实际视图移动量计算的
    let capture_region_x = region.capture_region_x;
    let capture_region_y = region.capture_region_y;
    let capture_offset_y = get_f64(settings, "capture_offset_y", 0.02);

    // 4. 执行滚轮 + 刷新
    AutoCapture::do_scroll_blocking(
        app_handle,
        is_running,
        scroll_mode.scroll_count,
        window_info.hwnd,
        window_info.left,
        window_info.top,
        drag_margin_bottom,
        drag_margin_left,
    )?;

    if !is_running.load(Ordering::SeqCst) {
        emit_log(app_handle, "warn", "截图已被用户停止");
        return Ok(Vec::new());
    }

    // 6. 蛇形遍历主循环
    let rows = region.grid_rows as u32;
    let cols = region.grid_cols as u32;
    let total = rows * cols;
    let drag_distance = (region.drag_x, region.drag_y);
    let mut screenshots: Vec<(u32, u32, image::RgbaImage)> = Vec::with_capacity(total as usize);
    let mut current: u32 = 0;

    emit_log(
        app_handle,
        "info",
        format!("开始蛇形遍历：{} 行 {} 列，共 {} 张", rows, cols, total),
    );

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

            // 截图前延迟（带 ±10% 随机，对应原 Python time.sleep(rand_delay(self.screenshot_delay))）
            std::thread::sleep(input::rand_delay(screenshot_delay));

            // 验证窗口有效性（对应原 Python 在循环中可能遇到的窗口消失）
            if !is_window_valid(window_info.hwnd) {
                emit_log(app_handle, "error", "游戏窗口已关闭");
                return Err(AppError::new("游戏窗口已关闭", "WINDOW_CLOSED"));
            }

            // 截图（capture_center_region 内部用 BitBlt，对应原 pyautogui.screenshot）
            let img = match screenshot::capture_center_region(
                window_info.hwnd,
                capture_region_x,
                capture_region_y,
                capture_offset_y,
            ) {
                Ok(img) => img,
                Err(e) => {
                    emit_log(
                        app_handle,
                        "error",
                        format!("截图失败 (行{}列{}): {}", row + 1, col + 1, e.message),
                    );
                    return Err(e);
                }
            };

            // 截图直接 push 到列表，全部完成后统一拼接
            screenshots.push((row, col, img));
            current += 1;

            // emit 进度 + 日志
            emit_progress(app_handle, current, total, row, col);
            emit_log(
                app_handle,
                "info",
                format!("已截图 {}/{} (行{}列{})", current, total, row + 1, col + 1),
            );

            // 行内移动（非最后一张）
            if idx < col_indices.len() - 1 {
                let direction = if is_even {
                    input::CameraDirection::Right
                } else {
                    input::CameraDirection::Left
                };
                emit_log(
                    app_handle,
                    "info",
                    format!(
                        "拖拽方向={:?} 距离=({}, {}) 时长={:.3}s",
                        direction, drag_distance.0, drag_distance.1, drag_duration
                    ),
                );
                move_camera_blocking(
                    direction,
                    drag_distance,
                    window_info.hwnd,
                    drag_margin_bottom,
                    drag_margin_left,
                    drag_duration,
                    stabilize_delay,
                );
                // 注意：input::move_camera 内部末尾已包含
                // thread::sleep(rand_delay(0.1 + stabilize_delay))，
                // 与原 Python move_camera 行为一致，此处不再额外 sleep。
            }
        }

        // 行末下移（非最后一行）
        if row < rows - 1 && is_running.load(Ordering::SeqCst) {
            emit_log(
                app_handle,
                "info",
                format!(
                    "下移：距离=({}, {}) 时长={:.3}s",
                    drag_distance.0, drag_distance.1, drag_duration
                ),
            );
            move_camera_blocking(
                input::CameraDirection::Down,
                drag_distance,
                window_info.hwnd,
                drag_margin_bottom,
                drag_margin_left,
                drag_duration,
                stabilize_delay,
            );
        }
    }

    Ok(screenshots)
}

/// 同步执行相机移动
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

    // 统一使用 SendInput + 中键拖拽
    input::move_camera(
        direction,
        drag_distance,
        client_rect.left,
        client_rect.top,
        client_rect.height,
        margin_bottom,
        margin_left,
        drag_duration,
        stabilize_delay,
    );
}

// 辅助函数

/// 验证窗口句柄是否有效
fn is_window_valid(hwnd_raw: isize) -> bool {
    let hwnd = HWND(hwnd_raw as *mut _);
    unsafe { IsWindow(hwnd) }.as_bool()
}

/// 从 `SettingsMap` 读取 `f64` 值（解析失败则返回 default）
fn get_f64(settings: &SettingsMap, key: &str, default: f64) -> f64 {
    settings
        .get(key)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(default)
}

// 事件推送

/// emit 进度事件
fn emit_progress(app_handle: &AppHandle, current: u32, total: u32, row: u32, col: u32) {
    let payload = ProgressPayload {
        current,
        total,
        row,
        col,
    };
    let _ = app_handle.emit("capture:progress", payload);
}

/// emit 状态事件（`capture:status`）
pub fn emit_status(app_handle: &AppHandle, is_running: bool, current: u32, total: u32, region: &str) {
    let payload = StatusPayload {
        is_running,
        current,
        total,
        region: region.to_string(),
    };
    let _ = app_handle.emit("capture:status", payload);
}