use std::collections::HashMap;

use tauri::State;

use crate::error::AppResult;
use crate::services::persistence::DbState;

/// 获取单个设置值
#[tauri::command]
pub fn get_setting(db: State<'_, DbState>, key: String) -> AppResult<Option<String>> {
    db.get_setting(&key)
}

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
