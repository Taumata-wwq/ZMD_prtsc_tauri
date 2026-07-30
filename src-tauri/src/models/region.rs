use serde::{Deserialize, Serialize};

/// 区域配置（基建 + 大地图）
/// 对应 SQLite region_config 表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConfig {
    pub id: Option<i64>,
    pub name: String,
    pub category: String,
    pub aspect_ratio: String,
    pub scroll_mode: String,
    pub grid_rows: i32,
    pub grid_cols: i32,
    pub overlap_x: f64,
    pub overlap_y: f64,
    pub drag_x: i32,
    pub drag_y: i32,
    /// 截图区域比例 X（统一 0.626，大地图 0.378）
    pub capture_region_x: f64,
    /// 截图区域比例 Y（统一 0.648，大地图 0.388）
    pub capture_region_y: f64,
    /// 截图偏移 Y（像素，默认 0）
    pub capture_offset_y: i32,
    /// 模板引用（如 "谷地-供能高地"）
    pub template_ref: Option<String>,
    /// 目标宽度（用户输入，0次记录的源真值）
    pub target_w: i32,
    /// 目标高度（用户输入，0次记录的源真值）
    pub target_h: i32,
    /// 大地图子地图名（如 "四号谷地"/"武陵"，仅 category="大地图" 时使用）
    pub sub_map: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}


