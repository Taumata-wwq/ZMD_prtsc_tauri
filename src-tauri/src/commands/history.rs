use tauri::State;

use crate::error::AppResult;
use crate::models::session::CaptureSession;
use crate::services::persistence::DbState;

#[tauri::command]
pub fn list_sessions(db: State<'_, DbState>, limit: Option<u32>) -> AppResult<Vec<CaptureSession>> {
    db.list_sessions(limit.unwrap_or(50))
}

#[tauri::command]
pub fn clear_history(db: State<'_, DbState>) -> AppResult<()> {
    db.clear_history()
}
