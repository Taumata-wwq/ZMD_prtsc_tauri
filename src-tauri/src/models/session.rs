use serde::{Deserialize, Serialize};

/// 截图会话状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Capturing,
    Completed,
    Interrupted,
    Error,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Capturing => "capturing",
            SessionStatus::Completed => "completed",
            SessionStatus::Interrupted => "interrupted",
            SessionStatus::Error => "error",
        }
    }
}

/// 裁剪框（相对原图像素）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropBox {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// 截图会话
/// 对应 SQLite capture_session 表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSession {
    pub id: Option<i64>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub region: Option<String>,
    pub scroll_mode: Option<String>,
    pub grid_rows: Option<i32>,
    pub grid_cols: Option<i32>,
    pub total_shots: Option<i32>,
    pub status: String,
    pub original_path: Option<String>,
    pub exported_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub crop_box: Option<String>,
    pub output_format: Option<String>,
    pub jpg_quality: Option<i32>,
}

impl CaptureSession {
    pub fn new(region: Option<String>, scroll_mode: Option<String>) -> Self {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        Self {
            id: None,
            started_at: now,
            finished_at: None,
            region,
            scroll_mode,
            grid_rows: None,
            grid_cols: None,
            total_shots: None,
            status: SessionStatus::Capturing.as_str().to_string(),
            original_path: None,
            exported_path: None,
            thumbnail_path: None,
            crop_box: None,
            output_format: None,
            jpg_quality: None,
        }
    }
}


