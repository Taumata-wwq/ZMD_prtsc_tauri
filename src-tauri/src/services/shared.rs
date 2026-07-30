//! 共享事件 payload 与发送函数：日志（`capture:log`）、状态（`capture:status`）

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// 日志事件 payload（`capture:log`）
#[derive(Debug, Clone, Serialize)]
pub struct LogPayload {
    /// 时间戳（本地时间，格式 `%Y-%m-%d %H:%M:%S`）
    pub timestamp: String,
    /// 日志内容
    pub message: String,
    /// 日志级别："info" / "warn" / "error"
    pub level: String,
}

/// 获取当前时间字符串（统一格式 `%Y-%m-%d %H:%M:%S`）
pub fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 发送日志事件到前端（`capture:log`）。emit 失败时打印到 stderr，不阻断调用流程。
pub fn emit_log(app: &AppHandle, level: &'static str, message: impl Into<String>) {
    let payload = LogPayload {
        timestamp: now_str(),
        message: message.into(),
        level: level.to_string(),
    };
    if let Err(e) = app.emit("capture:log", &payload) {
        eprintln!("发送日志事件失败: {}", e);
    }
}

/// 若目标路径已存在，则在文件名后追加 `_1` / `_2` / ... 直到不冲突为止。
///
/// 用于截图原图/导出图像保存时避免覆盖既有文件。
pub fn resolve_unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or(Path::new("."));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    let mut i = 1;
    loop {
        let new_path = parent.join(format!("{}_{}.{}", stem, i, ext));
        if !new_path.exists() {
            return new_path;
        }
        i += 1;
    }
}

/// 状态事件 payload（`capture:status`）
#[derive(serde::Serialize, Clone)]
struct StatusPayload {
    is_running: bool,
    current: u32,
    total: u32,
    region: String,
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
