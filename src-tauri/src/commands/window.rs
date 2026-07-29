use serde::Serialize;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::services::persistence::DbState;
use crate::services::game_window;

/// 游戏窗口信息
#[derive(Debug, Serialize)]
pub struct WindowInfo {
    pub title: String,
    pub hwnd: isize,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

/// 查找 Endfield 游戏窗口
#[tauri::command]
pub fn find_game_window() -> AppResult<Option<WindowInfo>> {
    match game_window::find_endfield_window() {
        Ok(Some(info)) => Ok(Some(WindowInfo {
            title: info.title,
            hwnd: info.hwnd as isize,
            left: info.left,
            top: info.top,
            width: info.width,
            height: info.height,
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(AppError::new(e.to_string(), "WINDOW_ERROR")),
    }
}

/// 激活并置顶游戏窗口
#[tauri::command]
pub fn activate_game_window() -> AppResult<()> {
    game_window::activate_window().map_err(|e| AppError::new(e.to_string(), "WINDOW_ERROR"))
}

/// 加载窗口状态
#[tauri::command]
pub fn load_window_state(db: State<'_, DbState>) -> AppResult<crate::models::setting::WindowState> {
    db.load_window_state()
}

/// 保存窗口状态
#[tauri::command]
pub fn save_window_state(
    db: State<'_, DbState>,
    state: crate::models::setting::WindowState,
) -> AppResult<()> {
    db.save_window_state(&state)
}