use std::collections::HashMap;

use tauri::State;

use crate::error::AppResult;
use crate::services::persistence::DbState;

/// 设置单个值
#[tauri::command]
pub fn set_setting(db: State<'_, DbState>, key: String, value: String) -> AppResult<()> {
    db.set_setting(&key, &value)
}

/// 获取全部设置
#[tauri::command]
pub fn get_all_settings(db: State<'_, DbState>) -> AppResult<HashMap<String, String>> {
    db.get_all_settings()
}

/// 批量设置多个值
#[tauri::command]
pub fn set_many_settings(db: State<'_, DbState>, entries: HashMap<String, String>) -> AppResult<()> {
    for (k, v) in entries {
        db.set_setting(&k, &v)?;
    }
    Ok(())
}

/// 重置所有数据
/// - 清空 region_config、app_setting、window_state，重新写入默认值
/// - 若 include_history=true，同时清空 capture_session 表
/// - 不删除磁盘文件，仅重置数据库
#[tauri::command]
pub fn reset_data(db: State<'_, DbState>, include_history: bool) -> AppResult<()> {
    db.reset_data(include_history)
}
