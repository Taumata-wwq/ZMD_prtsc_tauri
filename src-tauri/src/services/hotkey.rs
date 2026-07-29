//! 全局热键服务：F3 开始/停止截图，通过 emit "hotkey" 事件通知前端

use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::error::{AppError, AppResult};

/// 热键事件名称（前端通过 `listen("hotkey", ...)` 监听）
pub const HOTKEY_EVENT: &str = "hotkey";

/// 注册 F3 全局热键，仅在按下时触发
pub fn register_hotkeys(app: &AppHandle) -> AppResult<()> {
    let manager = app.global_shortcut();

    // F3：开始/停止截图（合并按钮，由前端按 isRunning 状态分发）
    manager
        .on_shortcut("F3", move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = app.emit(HOTKEY_EVENT, "F3");
            }
        })
        .map_err(|e| AppError::new(e.to_string(), "HOTKEY_REGISTER_ERROR"))?;

    Ok(())
}
