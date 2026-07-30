use tauri::State;

use crate::error::AppResult;
use crate::models::region::RegionConfig;
use crate::models::scroll_mode::{AllCountsResult, ScrollMode};
use crate::services::persistence::DbState;

/// 列出所有区域配置
#[tauri::command]
pub fn list_regions(db: State<'_, DbState>) -> AppResult<Vec<RegionConfig>> {
    db.list_regions()
}

/// 新增或更新区域配置（upsert）
#[tauri::command]
pub fn upsert_region(db: State<'_, DbState>, config: RegionConfig) -> AppResult<()> {
    db.upsert_region(&config)
}

/// 删除区域配置
#[tauri::command]
pub fn delete_region(
    db: State<'_, DbState>,
    name: String,
    aspect_ratio: String,
    scroll_mode: String,
) -> AppResult<()> {
    db.delete_region(&name, &aspect_ratio, &scroll_mode)
}

/// 列出所有滚动模式
#[tauri::command]
pub fn list_scroll_modes(db: State<'_, DbState>) -> AppResult<Vec<ScrollMode>> {
    db.list_scroll_modes()
}

/// 从 count=0 的 target 推导所有 0-8 次的数据
#[tauri::command]
pub async fn derive_all_counts(
    client_w: i32,
    client_h: i32,
    target_w: i32,
    target_h: i32,
    overlap_min: f64,
    overlap_max: f64,
) -> Result<Option<AllCountsResult>, String> {
    let result = crate::models::scroll_mode::derive_all_counts_from_base(
        client_w, client_h, target_w, target_h, overlap_min, overlap_max,
    );
    Ok(result)
}

/// 从 0次记录的 target 推导指定次数的完整 RegionConfig
/// 大地图/自定义直接返回数据库值，不参与推导
#[tauri::command]
pub fn derive_region_from_target(
    db: State<'_, DbState>,
    region_name: String,
    aspect_ratio: String,
    scroll_mode: String,
) -> AppResult<Option<RegionConfig>> {
    db.derive_region_from_target(&region_name, &aspect_ratio, &scroll_mode)
}
