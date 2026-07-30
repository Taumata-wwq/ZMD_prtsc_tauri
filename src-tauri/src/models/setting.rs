use std::collections::HashMap;

/// 全部应用设置的内存表示
pub type SettingsMap = HashMap<String, String>;

/// 默认应用设置键值
pub fn default_settings() -> Vec<(&'static str, &'static str)> {
    vec![
        ("theme", "dark"),
        ("language", "zh"),
        ("output_format", "JPG"),
        ("jpg_quality", "95"),
        ("original_folder", ""),
        ("screenshot_folder", ""),
        ("thumbnail_folder", ""),
        ("stabilize_delay", "0.13"),
        ("screenshot_delay", "0.13"),
        ("drag_duration", "0.07"),
        ("drag_margin_bottom", "10"),
        ("drag_margin_left", "10"),
        ("capture_offset_y", "0.02"),
        ("overlap_min", "0.0"),
        ("overlap_max", "0.5"),
        ("minimize_on_capture", "true"),
        ("filename_pattern", "{region}_{timestamp}_{scrollMode}"),
        ("last_region", "武陵-武陵城"),
        ("last_scroll_mode", "0次"),
        ("last_aspect_ratio", "16:9"),
        ("last_rows", "2"),
        ("last_cols", "2"),
    ]
}
