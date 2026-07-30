//! 截图命令：start/stop_capture、预览图存取

use std::sync::Arc;

use tauri::{AppHandle, State};
use tauri::ipc::Response;

use crate::error::{AppError, AppResult};
use crate::models::session::CaptureSession;
use crate::services::{capture_pipeline, emit_log};
use crate::services::auto_capture::AutoCapture;
use crate::services::persistence::DbState;

#[tauri::command]
pub async fn start_capture(
    app: AppHandle,
    db: State<'_, DbState>,
    auto_capture: State<'_, Arc<AutoCapture>>,
    region_name: String,
    scroll_mode: String,
    rows: Option<u32>,
    cols: Option<u32>,
) -> AppResult<i64> {
    if auto_capture.is_running() {
        return Err(AppError::new(
            "截图正在进行中，请先停止当前任务",
            "CAPTURE_IN_PROGRESS",
        ));
    }

    // 数据库只存 0次记录，指定次数的配置需实时推导
    let mut region = db
        .derive_region_from_target(&region_name, "16:9", &scroll_mode)?
        .ok_or_else(|| {
            AppError::new(
                format!(
                    "未找到区域配置: {}/{}/{}",
                    region_name, "16:9", scroll_mode
                ),
                "REGION_NOT_FOUND",
            )
        })?;

    if let (Some(r), Some(c)) = (rows, cols) {
        if r == 0 || c == 0 {
            return Err(AppError::new(
                format!("网格行列数必须大于 0（收到 {}x{}）", r, c),
                "INVALID_GRID",
            ));
        }
        region.grid_rows = r as i32;
        region.grid_cols = c as i32;
    }

    let scroll_modes = db.list_scroll_modes()?;
    let scroll_mode = scroll_modes
        .iter()
        .find(|s| s.name == region.scroll_mode)
        .cloned()
        .ok_or_else(|| {
            AppError::new(
                format!("未找到滚动模式: {}", region.scroll_mode),
                "SCROLL_MODE_NOT_FOUND",
            )
        })?;

    let settings = db.get_all_settings()?;

    let total = (region.grid_rows as u32) * (region.grid_cols as u32);
    let mut session = CaptureSession::new(
        Some(region.name.clone()),
        Some(region.scroll_mode.clone()),
    );
    session.grid_rows = Some(region.grid_rows);
    session.grid_cols = Some(region.grid_cols);
    session.total_shots = Some(total as i32);
    let session_id = db.insert_session(&session)?;

    emit_log(
        &app,
        "info",
        format!(
            "启动自动截图：区域={} 滚动模式={} 网格={}x{}",
            region.name, region.scroll_mode, region.grid_rows, region.grid_cols
        ),
    );

    // 截图保存在内存列表中，全部完成后统一拼接
    tokio::spawn(capture_pipeline::handle_capture_result(
        app.clone(),
        Arc::clone(&auto_capture),
        session_id,
        session,
        region,
        scroll_mode,
        settings,
    ));

    Ok(session_id)
}

#[tauri::command]
pub async fn stop_capture(
    app: AppHandle,
    auto_capture: State<'_, Arc<AutoCapture>>,
) -> AppResult<()> {
    if !auto_capture.is_running() {
        emit_log(&app, "warn", "当前没有正在进行的截图任务");
        return Ok(());
    }
    auto_capture.stop();
    emit_log(&app, "info", "已请求停止截图，等待当前操作完成...");
    Ok(())
}

/// 拉取预览图 PNG 字节流，空 Vec 表示暂无预览
#[tauri::command]
pub async fn get_preview_image(
    auto_capture: State<'_, Arc<AutoCapture>>,
) -> Result<Response, String> {
    match auto_capture.take_preview_png().await {
        // ipc::Response 以原始字节流传给前端，避免序列化为 number[] 的开销
        Some(bytes) => Ok(Response::new(bytes)),
        None => Ok(Response::new(Vec::new())),
    }
}

/// 拉取预览图磁盘路径（非消费式，可多次调用）
#[tauri::command]
pub async fn get_preview_path(
    auto_capture: State<'_, Arc<AutoCapture>>,
) -> AppResult<String> {
    Ok(auto_capture.get_preview_path().await.unwrap_or_default())
}

/// 设置预览图磁盘路径（用于从历史记录加载原图重新编辑）
#[tauri::command]
pub async fn set_preview_path(
    auto_capture: State<'_, Arc<AutoCapture>>,
    path: String,
) -> AppResult<()> {
    auto_capture.set_preview_path(path).await;
    Ok(())
}
