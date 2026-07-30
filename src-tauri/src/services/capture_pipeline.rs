//! 截图结果处理管线：拼接图像、编码保存、更新会话状态、推送事件
//!
//! 由 `commands::capture::start_capture` spawn 调用，与 command 层解耦。

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::error::{AppError, AppResult};
use crate::models::region::RegionConfig;
use crate::models::scroll_mode::ScrollMode;
use crate::models::session::{CaptureSession, SessionStatus};
use crate::models::setting::SettingsMap;
use crate::services::auto_capture::AutoCapture;
use crate::services::persistence::DbState;
use crate::services::shared::{emit_log, emit_status, now_str, resolve_unique_path};
use crate::services::stitcher;

/// 处理截图结果：拼接图像、编码保存、更新会话状态、推送事件
pub async fn handle_capture_result(
    app: AppHandle,
    auto_capture: Arc<AutoCapture>,
    session_id: i64,
    mut session: CaptureSession,
    region: RegionConfig,
    scroll_mode: ScrollMode,
    settings: SettingsMap,
) {
    let result = auto_capture
        .auto_capture_grid(&region, &scroll_mode, &settings)
        .await;

    match result {
        Ok(()) => {
            let screenshots = auto_capture.get_screenshots().await;
            let count = screenshots.len() as u32;

            if screenshots.is_empty() {
                emit_log(
                    &app,
                    "warn",
                    "未捕获到截图（可能已被用户中断）",
                );
                session.status = SessionStatus::Interrupted.as_str().to_string();
                session.finished_at = Some(now_str());
                let db_state = app.state::<DbState>();
                finalize_session_failure(&app, &db_state, session_id, &session, &region.name);
                return;
            }

            // 通知前端正在处理图像（显示加载指示器）
            let _ = app.emit("capture:processing", count);

            let grid = (region.grid_rows as u32, region.grid_cols as u32);
            let overlap = (region.overlap_x, region.overlap_y);
            let format = settings.get("output_format").map(|s| s.to_uppercase()).unwrap_or_else(|| "JPG".to_string());
            let quality: u8 = settings.get("jpg_quality").and_then(|s| s.parse().ok()).unwrap_or(95);
            let encode_format = format.clone();
            let encode_result = tokio::task::spawn_blocking(move || -> AppResult<(Vec<u8>, Vec<u8>)> {
                let stitched = stitcher::stitch_images(&screenshots, grid, overlap)?;
                let bytes = match encode_format.as_str() {
                    "PNG" => stitcher::encode_png(&stitched, true)?,
                    _ => stitcher::encode_jpeg(&stitched, quality)?,
                };
                let thumb_bytes = stitcher::generate_thumbnail(&stitched, 400)?;
                Ok((bytes, thumb_bytes))
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
                Ok((bytes, thumb_bytes)) => {
                    let app_data_dir = app.path().app_data_dir();
                    let (original_path_str, thumbnail_path_str) = match app_data_dir {
                        Ok(ref dir) => {
                            let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
                            let pattern = settings.get("filename_pattern").map(|s| s.as_str()).unwrap_or("{region}_{timestamp}_{scrollMode}");
                            let filename = pattern
                                .replace("{prefix}", "stitched")
                                .replace("{timestamp}", &ts)
                                .replace("{region}", &region.name)
                                .replace("{scrollMode}", &region.scroll_mode);
                            let ext = if format == "PNG" { "png" } else { "jpg" };
                            let original_folder = settings.get("original_folder")
                                .filter(|s| !s.is_empty())
                                .map(PathBuf::from)
                                .unwrap_or_else(|| dir.join("originals"));
                            let _ = fs::create_dir_all(&original_folder);
                            let original_path = resolve_unique_path(&original_folder.join(format!("{}.{}", filename, ext)));
                            match fs::write(&original_path, &bytes) {
                                Ok(_) => {
                                    let thumbnail_folder = settings.get("thumbnail_folder")
                                        .filter(|s| !s.is_empty())
                                        .map(PathBuf::from)
                                        .unwrap_or_else(|| dir.join("thumbnails"));
                                    let _ = fs::create_dir_all(&thumbnail_folder);
                                    let thumb_stem = original_path.file_stem().and_then(|s| s.to_str()).unwrap_or(&filename);
                                    let thumb_path = thumbnail_folder.join(format!("{}.jpg", thumb_stem));
                                    let thumb_str = match fs::write(&thumb_path, &thumb_bytes) {
                                        Ok(_) => Some(thumb_path.to_string_lossy().to_string()),
                                        Err(e) => {
                                            emit_log(&app, "warn", format!("保存缩略图失败: {}", e));
                                            None
                                        }
                                    };
                                    (Some(original_path.to_string_lossy().to_string()), thumb_str)
                                }
                                Err(e) => {
                                    emit_log(&app, "warn", format!("保存原图失败: {}", e));
                                    (None, None)
                                }
                            }
                        }
                        Err(ref e) => {
                            emit_log(&app, "warn", format!("获取 app_data_dir 失败: {}", e));
                            (None, None)
                        }
                    };

                    if let Some(p) = &original_path_str {
                        auto_capture.set_preview_path(p.clone()).await;
                    } else {
                        auto_capture.set_preview_png(bytes).await;
                    }

                    let _ = app.emit("capture:preview-ready", &original_path_str);
                    emit_log(&app, "info", "预览图已生成");
                    session.status = SessionStatus::Completed.as_str().to_string();
                    session.finished_at = Some(now_str());
                    session.original_path = original_path_str.clone();
                    session.thumbnail_path = thumbnail_path_str.clone();
                    let db_state = app.state::<DbState>();
                    let _ = db_state.update_session(session_id, &session);
                    emit_status(&app, false, count, count, &region.name);
                }
                Err(e) => {
                    emit_log(
                        &app,
                        "error",
                        format!("图像处理失败: {} ({})", e.message, e.code),
                    );
                    session.status = SessionStatus::Error.as_str().to_string();
                    session.finished_at = Some(now_str());
                    let db_state = app.state::<DbState>();
                    finalize_session_failure(&app, &db_state, session_id, &session, &region.name);
                }
            }
        }
        Err(e) => {
            emit_log(
                &app,
                "error",
                format!("截图失败: {} ({})", e.message, e.code),
            );
            session.status = SessionStatus::Error.as_str().to_string();
            session.finished_at = Some(now_str());
            let db_state = app.state::<DbState>();
            finalize_session_failure(&app, &db_state, session_id, &session, &region.name);
        }
    }
}

/// 处理截图失败时的会话状态更新和事件推送
fn finalize_session_failure(
    app: &AppHandle,
    db_state: &DbState,
    session_id: i64,
    session: &CaptureSession,
    region_name: &str,
) {
    let _ = db_state.update_session(session_id, session);
    emit_status(app, false, 0, 0, region_name);
}
