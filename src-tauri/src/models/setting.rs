use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// 全部应用设置的内存表示
pub type SettingsMap = HashMap<String, String>;

/// 默认应用设置键值
pub fn default_settings() -> Vec<(&'static str, &'static str)> {
    vec![
        ("theme", "dark"),
        ("language", "zh"),  // UI 语言，默认中文
        ("output_format", "JPG"),
        ("jpg_quality", "95"),
        ("output_folder", ""),  // 空=使用 app_data_dir/screenshots；避免 dev 模式下触发文件监视器
        ("stabilize_delay", "0.13"),   // 130ms
        ("screenshot_delay", "0.13"),  // 130ms
        ("drag_duration", "0.07"),   // 70ms
        ("drag_margin_bottom", "10"),
        ("drag_margin_left", "10"),
        ("capture_offset_y", "0.02"),
        // overlap 硬约束范围（用于 derive_from_base 夹紧行列数）
        ("overlap_min", "0.0"),   // 允许 0% 重叠
        ("overlap_max", "0.5"),   // 最大 50% 重叠
        // 开始截图后是否最小化窗口
        ("minimize_on_capture", "true"),
        // 自定义导出文件名格式
        // 可用占位符：{timestamp} {region} {scrollMode}
        ("filename_pattern", "{region}_{timestamp}_{scrollMode}"),
        ("last_region", "武陵-武陵城"),
        ("last_scroll_mode", "0次"),
        ("last_aspect_ratio", "16:9"),
        ("last_rows", "2"),
        ("last_cols", "2"),
    ]
}

/// 默认区域配置列表
/// 返回 (name, category, scroll_mode, grid_rows, grid_cols, overlap_x, overlap_y,
///         drag_x, drag_y, capture_region_x, capture_region_y, capture_offset_y,
///         template_ref, target_w, target_h)
pub fn default_regions() -> Vec<(&'static str, &'static str, &'static str, i32, i32, f64, f64, i32, i32, f64, f64, i32, Option<&'static str>, i32, i32)> {
    vec![
        // 武陵-武陵城 (target: 10000 × 10000)
        ("武陵-武陵城", "武陵", "0次", 15, 9, 0.09, 0.01, 826, 523, 0.626, 0.648, 0, None, 10000, 10000),
        // 武陵-景玉谷 (target: 7200 × 7200)
        ("武陵-景玉谷", "武陵", "0次", 10, 6, 0.001, 0.01, 903, 522, 0.626, 0.648, 0, None, 7200, 7200),
        // 武陵-首敦 (target: 7200 × 7200)
        ("武陵-首敦", "武陵", "0次", 10, 6, 0.001, 0.01, 903, 522, 0.626, 0.648, 0, None, 7200, 7200),
        // 武陵-应龙关 (target: 7200 × 7200)
        ("武陵-应龙关", "武陵", "0次", 10, 6, 0.001, 0.01, 903, 522, 0.626, 0.648, 0, None, 7200, 7200),
        // 四号谷地-枢纽区 (target: 9000 × 9000)
        ("四号谷地-枢纽区", "四号谷地", "0次", 13, 8, 0.06, 0.01, 851, 522, 0.626, 0.648, 0, None, 9000, 9000),
        // 四号谷地-供能高地 (target: 6200 × 6200)
        ("四号谷地-供能高地", "四号谷地", "0次", 9, 5, 0.001, 0.008, 905, 524, 0.626, 0.648, 0, None, 6200, 6200),
        // 四号谷地-谷地通道 (target: 6200 × 6200)
        ("四号谷地-谷地通道", "四号谷地", "0次", 9, 5, 0.001, 0.008, 905, 524, 0.626, 0.648, 0, None, 6200, 6200),
        // 四号谷地-源石研究园 (target: 6200 × 6200)
        ("四号谷地-源石研究园", "四号谷地", "0次", 9, 5, 0.001, 0.008, 905, 524, 0.626, 0.648, 0, None, 6200, 6200),
        // 大地图（capture_region_x=0.378, capture_region_y=0.388）
        ("大地图", "大地图", "0次", 11, 9, 0.001, 0.001, 905, 525, 0.378, 0.388, 0, None, 0, 0),
        // 自定义（仅 0次）
        ("自定义", "自定义", "0次", 2, 2, 0.001, 0.001, 905, 525, 0.626, 0.648, 0, None, 0, 0),
    ]
}