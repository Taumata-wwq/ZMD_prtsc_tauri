//! 日志事件 payload 与发送函数

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

/// 发送日志事件到前端（`capture:log`）
///
/// 统一 `commands::capture` 和 `services::auto_capture` 中的日志发送逻辑。
/// emit 失败时打印到 stderr，不阻断调用流程。
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
