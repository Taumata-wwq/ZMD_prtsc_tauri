//! 窗口状态持久化：恢复位置/大小/置顶，监听移动/缩放事件防抖保存

use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Size, WindowEvent};
use tokio::sync::Mutex as TokioMutex;
use tokio::time::sleep;

use crate::error::{AppError, AppResult};
use crate::models::window_state::WindowState;
use crate::services::persistence::DbState;

/// 防抖延迟：500ms 内的连续事件只保存最后一次
const DEBOUNCE_INTERVAL: Duration = Duration::from_millis(500);

/// 将 `tauri::Error` 转换为 `AppError`（窗口操作错误统一标记为 WINDOW_ERROR）
fn map_window_err(e: tauri::Error) -> AppError {
    AppError::new(e.to_string(), "WINDOW_ERROR")
}

/// 检查屏幕坐标点是否在任何显示器的工作区域内
/// 使用 Win32 `MonitorFromPoint` + `GetMonitorInfoW` 判断，不在则返回 false（应重新居中窗口）。
fn is_point_on_screen(x: i32, y: i32) -> bool {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITOR_DEFAULTTONULL, MONITORINFO,
    };

    unsafe {
        let pt = POINT { x, y };
        let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONULL);
        if hmon.is_invalid() {
            return false;
        }
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut info).as_bool() {
            // 工作区域（排除任务栏）
            let rc = info.rcWork;
            x >= rc.left && x < rc.right && y >= rc.top && y < rc.bottom
        } else {
            false
        }
    }
}

/// 启动时从数据库恢复窗口状态
/// 恢复顺序：最大化 → 位置（屏幕内则恢复，否则居中）→ 大小 → 置顶（独立于最大化）。
pub fn restore_window_state(app: &AppHandle) -> AppResult<()> {
    let db = app.state::<DbState>();
    let state = db.load_window_state()?;

    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };

    if state.is_maximized {
        let _ = window.maximize();
    } else {
        // 检查保存的位置是否在屏幕内，不在则居中
        let pos_valid = matches!((state.x, state.y), (Some(x), Some(y)) if is_point_on_screen(x, y));
        if pos_valid {
            // pos_valid 已确认 x/y 为 Some，unwrap_or(0) 防 NULL 字段 panic
            let _ = window.set_position(Position::Physical(PhysicalPosition::new(
                state.x.unwrap_or(0),
                state.y.unwrap_or(0),
            )));
        } else {
            let _ = window.center();
        }
        if let (Some(w), Some(h)) = (state.width, state.height) {
            let _ = window.set_size(Size::Physical(PhysicalSize::new(w as u32, h as u32)));
        }
    }

    let _ = window.set_always_on_top(state.always_on_top);

    Ok(())
}

/// 初始化窗口事件监听（在 lib.rs setup 中调用）
/// `Moved`/`Resized` 触发防抖保存；`CloseRequested` 立即保存。不监听 `Destroyed`（API 可能已失效）。
pub fn init_window_state_listener(app: &AppHandle) -> AppResult<()> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::new("主窗口未找到", "WINDOW_ERROR"))?;

    let app_handle = app.clone();
    let last_event: Arc<TokioMutex<Option<Instant>>> = Arc::new(TokioMutex::new(None));

    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
            let app = app_handle.clone();
            let last = last_event.clone();
            tauri::async_runtime::spawn(async move {
                save_debounced(app, last).await;
            });
        } else if matches!(event, WindowEvent::CloseRequested { .. }) {
            // 关闭时立即保存（不防抖）
            let app = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let _ = save_current_window_state(&app);
            });
        }
    });

    Ok(())
}

/// 防抖保存窗口状态（trailing debounce）
/// 进入时记录时间戳到共享 `last`，等待 500ms 后检查是否仍是最后一次事件，是则保存。
async fn save_debounced(app: AppHandle, last: Arc<TokioMutex<Option<Instant>>>) {
    let my_time = Instant::now();
    {
        let mut guard = last.lock().await;
        *guard = Some(my_time);
    }

    sleep(DEBOUNCE_INTERVAL).await;

    let is_latest = {
        let guard = last.lock().await;
        matches!(*guard, Some(t) if t == my_time)
    };

    if is_latest {
        let _ = save_current_window_state(&app);
    }
}

/// 保存当前窗口状态到数据库（同步，因 `DbState::save_window_state` 同步）
fn save_current_window_state(app: &AppHandle) -> AppResult<()> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::new("主窗口未找到", "WINDOW_ERROR"))?;

    let position = window.outer_position().map_err(map_window_err)?;
    let size = window.outer_size().map_err(map_window_err)?;
    let is_maximized = window.is_maximized().map_err(map_window_err)?;
    let always_on_top = window.is_always_on_top().map_err(map_window_err)?;

    let state = WindowState {
        x: Some(position.x),
        y: Some(position.y),
        width: Some(size.width as i32),
        height: Some(size.height as i32),
        is_maximized,
        always_on_top,
        updated_at: chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    };

    let db = app.state::<DbState>();
    db.save_window_state(&state)?;

    Ok(())
}
