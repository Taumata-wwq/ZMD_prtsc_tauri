use std::fs;

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::models::session::CaptureSession;
use crate::services::persistence::DbState;

/// 列出所有历史会话
#[tauri::command]
pub fn list_sessions(db: State<'_, DbState>) -> AppResult<Vec<CaptureSession>> {
    db.list_sessions()
}

/// 删除一条历史记录：
///   - 删除数据库记录
///   - 删除该 session 的缩略图文件
///   - delete_original=true 时删除该 session 的原图文件
///   - delete_screenshot=true 时删除该 session 的所有截图文件（exported_path 换行分隔）
#[tauri::command]
pub fn delete_session(
    db: State<'_, DbState>,
    session_id: i64,
    delete_original: bool,
    delete_screenshot: bool,
) -> AppResult<()> {
    let session = db
        .get_session(session_id)?
        .ok_or_else(|| AppError::new("会话不存在", "SESSION_NOT_FOUND"))?;

    if let Some(p) = session.thumbnail_path.as_ref().filter(|s| !s.is_empty()) {
        let _ = fs::remove_file(p);
    }

    if delete_original {
        if let Some(p) = session.original_path.as_ref().filter(|s| !s.is_empty()) {
            let _ = fs::remove_file(p);
        }
    }

    // 截图路径可能多个（换行分隔）
    if delete_screenshot {
        if let Some(paths) = session.exported_path.as_ref().filter(|s| !s.is_empty()) {
            for line in paths.lines() {
                let line = line.trim();
                if !line.is_empty() {
                    let _ = fs::remove_file(line);
                }
            }
        }
    }

    db.delete_session(session_id)?;
    Ok(())
}
