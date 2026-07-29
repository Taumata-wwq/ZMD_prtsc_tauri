use tauri::State;

use crate::error::AppResult;
use crate::models::region::RegionConfig;
use crate::models::scroll_mode::ScrollMode;
use crate::services::persistence::DbState;

/// 列出所有区域配置
#[tauri::command]
pub fn list_regions(db: State<'_, DbState>) -> AppResult<Vec<RegionConfig>> {
    db.list_regions()
}

/// 按 name + aspect_ratio + scroll_mode 查询单个区域配置
#[tauri::command]
pub fn get_region(
    db: State<'_, DbState>,
    name: String,
    aspect_ratio: String,
    scroll_mode: String,
) -> AppResult<Option<RegionConfig>> {
    db.get_region(&name, &aspect_ratio, &scroll_mode)
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

// ============ 区域参数推导 ============

/// 通解推导结果 DTO
#[derive(serde::Serialize)]
pub struct DerivedParamsDto {
    pub img_w: i32,
    pub img_h: i32,
    pub drag_x: i32,
    pub drag_y: i32,
    pub capture_region_x: f64,
    pub capture_region_y: f64,
}

/// 根据客户端窗口尺寸、滚动模式与 overlap 推导区域参数
/// 未知 scroll_mode 返回 Ok(None) 而非 Err
#[tauri::command]
pub async fn derive_region_params(
    client_w: i32,
    client_h: i32,
    scroll_mode: String,
    overlap_x: f64,
    overlap_y: f64,
) -> Result<Option<DerivedParamsDto>, String> {
    let result = crate::models::scroll_mode::derive_region_params(
        client_w, client_h, &scroll_mode, overlap_x, overlap_y,
    );
    Ok(result.map(|p| DerivedParamsDto {
        img_w: p.img_w,
        img_h: p.img_h,
        drag_x: p.drag_x,
        drag_y: p.drag_y,
        capture_region_x: p.capture_region_x,
        capture_region_y: p.capture_region_y,
    }))
}

// ============ 基于基准数据的推导 ============

/// 基于基准数据推导结果 DTO
#[derive(serde::Serialize)]
pub struct DerivedFromBaseDto {
    pub img_w: i32,
    pub img_h: i32,
    pub target_w: i32,
    pub target_h: i32,
    pub actual_rows: i32,
    pub actual_cols: i32,
    pub drag_x: i32,
    pub drag_y: i32,
    pub overlap_x: f64,
    pub overlap_y: f64,
    pub clamped: bool,
}

/// 基于 (base_rows, base_cols, base_overlap) 反推目标总尺寸并重算参数
#[tauri::command]
pub async fn derive_from_base(
    client_w: i32,
    client_h: i32,
    scroll_mode: String,
    base_rows: i32,
    base_cols: i32,
    base_overlap_x: f64,
    base_overlap_y: f64,
    target_rows: i32,
    target_cols: i32,
    overlap_min: f64,
    overlap_max: f64,
) -> Result<Option<DerivedFromBaseDto>, String> {
    let result = crate::models::scroll_mode::derive_from_base(
        client_w,
        client_h,
        &scroll_mode,
        base_rows,
        base_cols,
        base_overlap_x,
        base_overlap_y,
        target_rows,
        target_cols,
        overlap_min,
        overlap_max,
    );
    Ok(result.map(|p| DerivedFromBaseDto {
        img_w: p.img_w,
        img_h: p.img_h,
        target_w: p.target_w,
        target_h: p.target_h,
        actual_rows: p.actual_rows,
        actual_cols: p.actual_cols,
        drag_x: p.drag_x,
        drag_y: p.drag_y,
        overlap_x: p.overlap_x,
        overlap_y: p.overlap_y,
        clamped: p.clamped,
    }))
}

/// 基于用户指定的 target_w/target_h 推导区域参数
/// 未知 scroll_mode 或 target < img 时返回 Ok(None)
#[tauri::command]
pub async fn derive_from_target(
    client_w: i32,
    client_h: i32,
    scroll_mode: String,
    target_w: i32,
    target_h: i32,
    overlap_min: f64,
    overlap_max: f64,
) -> Result<Option<DerivedFromBaseDto>, String> {
    let result = crate::models::scroll_mode::derive_from_target(
        client_w,
        client_h,
        &scroll_mode,
        target_w,
        target_h,
        overlap_min,
        overlap_max,
    );
    Ok(result.map(|p| DerivedFromBaseDto {
        img_w: p.img_w,
        img_h: p.img_h,
        target_w: p.target_w,
        target_h: p.target_h,
        actual_rows: p.actual_rows,
        actual_cols: p.actual_cols,
        drag_x: p.drag_x,
        drag_y: p.drag_y,
        overlap_x: p.overlap_x,
        overlap_y: p.overlap_y,
        clamped: p.clamped,
    }))
}

// ============ 0-8 次推导 ============

/// 9 个滚动次数推导结果 DTO
#[derive(serde::Serialize)]
pub struct AllCountsResultDto {
    pub counts: Vec<DerivedFromBaseDto>,
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
) -> Result<Option<AllCountsResultDto>, String> {
    let result = crate::models::scroll_mode::derive_all_counts_from_base(
        client_w, client_h, target_w, target_h, overlap_min, overlap_max,
    );
    Ok(result.map(|r| {
        fn to_dto(p: crate::models::scroll_mode::DerivedFromBase) -> DerivedFromBaseDto {
            DerivedFromBaseDto {
                img_w: p.img_w, img_h: p.img_h, target_w: p.target_w, target_h: p.target_h,
                actual_rows: p.actual_rows, actual_cols: p.actual_cols,
                drag_x: p.drag_x, drag_y: p.drag_y,
                overlap_x: p.overlap_x, overlap_y: p.overlap_y, clamped: p.clamped,
            }
        }
        AllCountsResultDto {
            counts: r.counts.into_iter().map(to_dto).collect(),
        }
    }))
}

// ============ 实时推导 ============

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
