use serde::{Deserialize, Serialize};

/// 窗口状态
/// 对应 SQLite window_state 表（单行表，id=1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub is_maximized: bool,
    pub always_on_top: bool,
    pub updated_at: String,
}

impl Default for WindowState {
    fn default() -> Self {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        Self {
            x: None,
            y: None,
            width: Some(900),
            height: Some(600),
            is_maximized: false,
            always_on_top: false,
            updated_at: now,
        }
    }
}
