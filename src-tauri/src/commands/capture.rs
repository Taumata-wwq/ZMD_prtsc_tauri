//! 截图命令：start_capture / stop_capture / get_capture_status

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri::ipc::Response;
use tokio::sync::Mutex as TokioMutex;

use crate::error::{AppError, AppResult};
use crate::models::session::{CaptureSession, SessionStatus};
use crate::services::{emit_log, now_str};
use crate::services::auto_capture::{self, AutoCapture};
use crate::services::persistence::DbState;
use crate::services::stitcher;

// =========================================================================
// Managed State
// =========================================================================

pub struct CaptureState {
    pub current: AtomicU32,
    pub total: AtomicU32,
    pub region_name: TokioMutex<String>,
}

impl Default for CaptureState {
    fn default() -> Self {
        Self {
            current: AtomicU32::new(0),
            total: AtomicU32::new(0),
            region_name: TokioMutex::new(String::new()),
        }
    }
}

/// `get_capture_status` 返回值
#[derive(Serialize)]
pub struct CaptureStatus {
    pub is_running: bool,
    pub current: u32,
    pub total: u32,
    pub region: String,
}

// =========================================================================
// Commands
// =========================================================================

#[tauri::command]
pub async fn start_capture(
    app: AppHandle,
    db: State<'_, DbState>,
    auto_capture: State<'_, Arc<AutoCapture>>,
    capture_state: State<'_, CaptureState>,
    region_name: String,
    scroll_mode: String,
    rows: Option<u32>,
    cols: Option<u32>,
) -> AppResult<i64> {
    // 1. 检查是否已在截图
    if auto_capture.is_running() {
        return Err(AppError::new(
            "截图正在进行中，请先停止当前任务",
            "CAPTURE_IN_PROGRESS",
        ));
    }

    // 2. 从 0次记录实时推导指定次数的完整配置
    //    新架构：数据库只存 0次记录，其他次数运行时推导
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

    // 3. 可选网格覆盖（自定义网格）
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

    // 4. 查询滚动模式
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

    // 5. 查询所有设置
    let settings = db.get_all_settings()?;

    // 6. 更新 capture_state
    let total = (region.grid_rows as u32) * (region.grid_cols as u32);
    capture_state.total.store(total, Ordering::SeqCst);
    capture_state.current.store(0, Ordering::SeqCst);
    *capture_state.region_name.lock().await = region_name.clone();

    // 7. 创建并插入 session（status = "capturing"）
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

    // 截图直接保存在内存列表中，全部完成后统一拼接

    // 8. spawn 任务执行截图（不阻塞 command 返回）
    let app_clone = app.clone();
    let auto_capture_arc = Arc::clone(&auto_capture);
    let region_c = region.clone();
    let scroll_mode_c = scroll_mode.clone();
    let settings_c = settings.clone();

    tokio::spawn(async move {
        let result = auto_capture_arc
            .auto_capture_grid(&region_c, &scroll_mode_c, &settings_c)
            .await;

        // session 在闭包内修改后写回 DB
        let mut session = session;

        match result {
            Ok(()) => {
                // 获取截图列表
                let screenshots = auto_capture_arc.get_screenshots().await;
                let count = screenshots.len() as u32;

                // 更新 capture_state.current（最终值，实时进度由事件推送）
                {
                    let capture_state_ref = app_clone.state::<CaptureState>();
                    capture_state_ref.current.store(count, Ordering::SeqCst);
                }

                if screenshots.is_empty() {
                    emit_log(
                        &app_clone,
                        "warn",
                        "未捕获到截图（可能已被用户中断）",
                    );
                    session.status = SessionStatus::Interrupted.as_str().to_string();
                    session.finished_at = Some(now_str());
                    let db_state = app_clone.state::<DbState>();
                    let _ = db_state.update_session(session_id, &session);
                    // 通知前端截图结束
                    auto_capture::emit_status(&app_clone, false, 0, 0, &region_c.name);
                    return;
                }

                // 通知前端正在处理图像（显示加载指示器）
                let _ = app_clone.emit("capture:processing", count);

                // 将拼接+编码放入 spawn_blocking，避免阻塞 tokio 运行时
                // 大图用 JPEG 编码（比 PNG 快 10-50 倍），小图用 PNG
                let grid = (region_c.grid_rows as u32, region_c.grid_cols as u32);
                let overlap = (region_c.overlap_x, region_c.overlap_y);
                let encode_result = tokio::task::spawn_blocking(move || {
                    let stitched = stitcher::stitch_images(&screenshots, grid, overlap)?;
                    let total_pixels = (stitched.width() * stitched.height()) as u64;
                    // 超过 400 万像素（约 1920x2160）用 JPEG，否则用 PNG
                    if total_pixels > 4_000_000 {
                        let bytes = stitcher::encode_jpeg_fast(&stitched, 90)?;
                        Ok::<(Vec<u8>, &str), AppError>((bytes, "jpg"))
                    } else {
                        let bytes = stitcher::encode_png_fast(&stitched)?;
                        Ok::<(Vec<u8>, &str), AppError>((bytes, "png"))
                    }
                })
                .await;

                let encode_result = match encode_result {
                    Ok(r) => r,
                    Err(e) => Err(AppError::new(
                        format!("拼接任务执行失败: {}", e),
                        "TASK_JOIN_ERROR",
                    )),
                };

                match encode_result {
                    Ok((bytes, ext)) => {
                        // 将拼接后的图片保存到 session 目录
                        let app_data_dir = app_clone
                            .path()
                            .app_data_dir()
                            .map_err(|e| AppError::new(format!("无法获取 app_data_dir: {}", e), "PATH_ERROR"));
                        let original_path_str = match app_data_dir {
                            Ok(dir) => {
                                let session_dir: PathBuf = dir.join("sessions").join(session_id.to_string());
                                let _ = fs::create_dir_all(&session_dir);
                                let filename = format!("original.{}", ext);
                                let original_path = session_dir.join(&filename);
                                match fs::write(&original_path, &bytes) {
                                    Ok(_) => Some(original_path.to_string_lossy().to_string()),
                                    Err(e) => {
                                        emit_log(&app_clone, "warn", format!("保存 {} 失败: {}", filename, e));
                                        None
                                    }
                                }
                            }
                            Err(e) => {
                                emit_log(&app_clone, "warn", format!("获取 app_data_dir 失败: {}", e.message));
                                None
                            }
                        };

                        // 路径保存成功时不存储 PNG 字节流，节省约 20MB 内存
                        // 前端通过 convertFileSrc(path) 直接加载磁盘文件，不需要字节流
                        // 仅在路径保存失败时才存字节流（后备方案 getPreviewImage）
                        if let Some(p) = &original_path_str {
                            auto_capture_arc.set_preview_path(p.clone()).await;
                        } else {
                            // 路径保存失败，存字节流作为后备
                            auto_capture_arc.set_preview_png(bytes).await;
                        }

                        // emit 事件携带磁盘路径，前端用 convertFileSrc 直接加载
                        // 避免 ipc::Response ArrayBuffer 传输可能的问题
                        let _ = app_clone.emit("capture:preview-ready", &original_path_str);
                        emit_log(&app_clone, "info", "预览图已生成");
                        session.status = SessionStatus::Completed.as_str().to_string();
                        session.finished_at = Some(now_str());
                        let db_state = app_clone.state::<DbState>();
                        // 同步回写 original_path（若保存成功）
                        if let Some(p) = &original_path_str {
                            let _ = db_state.update_session_paths(session_id, Some(p), None, None);
                        }
                        let _ = db_state.update_session(session_id, &session);
                        // 预览图已就绪后才通知前端截图结束
                        auto_capture::emit_status(&app_clone, false, count, count, &region_c.name);
                    }
                    Err(e) => {
                        emit_log(
                            &app_clone,
                            "error",
                            format!("图像处理失败: {} ({})", e.message, e.code),
                        );
                        session.status = SessionStatus::Error.as_str().to_string();
                        session.finished_at = Some(now_str());
                        let db_state = app_clone.state::<DbState>();
                        let _ = db_state.update_session(session_id, &session);
                        // 通知前端截图结束
                        auto_capture::emit_status(&app_clone, false, 0, 0, &region_c.name);
                    }
                }
            }
            Err(e) => {
                emit_log(
                    &app_clone,
                    "error",
                    format!("截图失败: {} ({})", e.message, e.code),
                );
                session.status = SessionStatus::Error.as_str().to_string();
                session.finished_at = Some(now_str());
                let db_state = app_clone.state::<DbState>();
                let _ = db_state.update_session(session_id, &session);
                // 通知前端截图结束
                auto_capture::emit_status(&app_clone, false, 0, 0, &region_c.name);
            }
        }
    });

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

#[tauri::command]
pub async fn get_capture_status(
    auto_capture: State<'_, Arc<AutoCapture>>,
    capture_state: State<'_, CaptureState>,
) -> AppResult<CaptureStatus> {
    Ok(CaptureStatus {
        is_running: auto_capture.is_running(),
        current: capture_state.current.load(Ordering::SeqCst),
        total: capture_state.total.load(Ordering::SeqCst),
        region: capture_state.region_name.lock().await.clone(),
    })
}

/// 拉取预览图 PNG 字节流。返回 byteLength === 0 表示暂无预览。
#[tauri::command]
pub async fn get_preview_image(
    auto_capture: State<'_, Arc<AutoCapture>>,
) -> Result<Response, String> {
    match auto_capture.take_preview_png().await {
        Some(bytes) => {
            // ipc::Response 从 Vec<u8> 构造，Tauri 以原始字节流传输给前端
            // 前端通过 invoke 收到 ArrayBuffer（而非 number[]），性能大幅提升
            Ok(Response::new(bytes))
        }
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
