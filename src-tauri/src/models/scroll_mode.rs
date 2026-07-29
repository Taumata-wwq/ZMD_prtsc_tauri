use serde::{Deserialize, Serialize};

/// 滚动次数（替代原"滚动模式"）
/// 对应 SQLite scroll_mode 表（保留表名兼容，内容改为 0-8 次）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollMode {
    pub id: Option<i64>,
    pub name: String,
    pub scroll_count: i32,
    pub is_default: bool,
}

/// 默认滚动次数列表（0-8 次）
/// scroll_count 控制游戏内滚轮滚动次数，影响地图缩放级别
/// 0 次 = 距地面最近，视野最小；8 次 = 距地面最远，视野最大
pub const SCROLL_MODES: &[(&str, i32, bool)] = &[
    ("0次", 0, true),
    ("1次", 1, false),
    ("2次", 2, false),
    ("3次", 3, false),
    ("4次", 4, false),
    ("5次", 5, false),
    ("6次", 6, false),
    ("7次", 7, false),
    ("8次", 8, false),
];

// ============ 滚动次数常量表（0-8） ============

/// 截图区域比例（所有次数统一）
const CAPTURE_REGION_X: f64 = 0.626;
const CAPTURE_REGION_Y: f64 = 0.648;

/// 视图移动倍率 rate（step = drag × rate）
const COUNT_RATES: [f64; 9] = [
    1.333, // 0
    1.213, // 1
    1.131, // 2
    1.066, // 3
    1.015, // 4
    0.974, // 5
    0.941, // 6
    0.912, // 7
    0.891, // 8
];

/// 拖拽距离最大值约束（基于 1280×720 最小支持画面）
const COUNT_DRAG_MAX: [(i32, i32); 9] = [
    (905, 525),   // 0
    (971, 563),   // 1
    (1037, 602),  // 2
    (1103, 640),  // 3
    (1169, 678),  // 4
    (1235, 716),  // 5
    (1276, 716),  // 6
    (1276, 716),  // 7
    (1276, 716),  // 8
];

/// target 比例常量 k（k[count] = target_count / target_0）
const COUNT_K: [(f64, f64); 9] = [
    (1.000, 1.000), // 0
    (0.820, 0.850), // 1
    (0.690, 0.740), // 2
    (0.600, 0.650), // 3
    (0.517, 0.565), // 4
    (0.455, 0.505), // 5
    (0.409, 0.457), // 6
    (0.369, 0.419), // 7
    (0.333, 0.370), // 8
];

/// 从 scroll_mode 字符串解析 scroll_count
/// 格式为 "0次"→0, "5次"→5, ...
pub fn parse_scroll_count(scroll_mode: &str) -> Option<i32> {
    scroll_mode
        .strip_suffix("次")
        .and_then(|s| s.parse().ok())
        .filter(|&c: &i32| (0..=8).contains(&c))
}

#[derive(Debug, Clone, Copy)]
pub struct ModeConstants {
    pub capture_region_x: f64,
    pub capture_region_y: f64,
    pub rate_x: f64,
    pub rate_y: f64,
}

/// 根据滚动次数获取基准截图区域常量
/// 未知次数返回 None
pub fn get_mode_constants(scroll_mode: &str) -> Option<ModeConstants> {
    let count = parse_scroll_count(scroll_mode)?;
    let rate = COUNT_RATES.get(count as usize)?;
    Some(ModeConstants {
        capture_region_x: CAPTURE_REGION_X,
        capture_region_y: CAPTURE_REGION_Y,
        rate_x: *rate,
        rate_y: *rate,
    })
}

/// 根据滚动次数获取 drag_max（直接用 i32 参数）
pub fn get_drag_max_by_count(count: i32) -> Option<(i32, i32)> {
    COUNT_DRAG_MAX.get(count as usize).copied()
}

// ============ 通解推导 ============

#[derive(Debug, Clone, Copy)]
pub struct DerivedParams {
    pub img_w: i32,
    pub img_h: i32,
    pub drag_x: i32,
    pub drag_y: i32,
    pub capture_region_x: f64,
    pub capture_region_y: f64,
}

pub fn derive_region_params(
    client_w: i32,
    client_h: i32,
    scroll_mode: &str,
    overlap_x: f64,
    overlap_y: f64,
) -> Option<DerivedParams> {
    let consts = get_mode_constants(scroll_mode)?;
    let img_w = (client_w as f64 * consts.capture_region_x).round() as i32;
    let img_h = (client_h as f64 * consts.capture_region_y).round() as i32;
    let step_x = img_w as f64 * (1.0 - overlap_x);
    let step_y = img_h as f64 * (1.0 - overlap_y);
    let drag_x = (step_x / consts.rate_x).round() as i32;
    let drag_y = (step_y / consts.rate_y).round() as i32;
    Some(DerivedParams {
        img_w,
        img_h,
        drag_x,
        drag_y,
        capture_region_x: consts.capture_region_x,
        capture_region_y: consts.capture_region_y,
    })
}

// ============ 基于基准数据的通解推导 ============

pub fn get_drag_max(scroll_mode: &str) -> Option<(i32, i32)> {
    let count = parse_scroll_count(scroll_mode)?;
    get_drag_max_by_count(count)
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct DerivedFromBase {
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

pub fn derive_from_base(
    client_w: i32,
    client_h: i32,
    scroll_mode: &str,
    base_rows: i32,
    base_cols: i32,
    base_overlap_x: f64,
    base_overlap_y: f64,
    target_rows: i32,
    target_cols: i32,
    overlap_min: f64,
    overlap_max: f64,
) -> Option<DerivedFromBase> {
    let consts = get_mode_constants(scroll_mode)?;
    let img_w = (client_w as f64 * consts.capture_region_x).round() as i32;
    let img_h = (client_h as f64 * consts.capture_region_y).round() as i32;

    let ovlp_px_x = (img_w as f64 * base_overlap_x) as i32;
    let ovlp_px_y = (img_h as f64 * base_overlap_y) as i32;
    let base_step_x = img_w - ovlp_px_x;
    let base_step_y = img_h - ovlp_px_y;
    let target_w = base_step_x * base_cols + ovlp_px_x;
    let target_h = base_step_y * base_rows + ovlp_px_y;

    let span_w = (target_w - img_w) as f64;
    let span_h = (target_h - img_h) as f64;
    let img_w_f = img_w as f64;
    let img_h_f = img_h as f64;

    let (mode_max_x, mode_max_y) = get_drag_max(scroll_mode)?;
    let step_max_x = (mode_max_x as f64 * consts.rate_x * 1.005).min(img_w_f * (1.0 - overlap_min));
    let step_max_y = (mode_max_y as f64 * consts.rate_y * 1.005).min(img_h_f * (1.0 - overlap_min));
    let step_min_x = img_w_f * (1.0 - overlap_max);
    let step_min_y = img_h_f * (1.0 - overlap_max);

    let cols_min = if span_w > 0.0 && step_max_x > 0.0 {
        (span_w / step_max_x).ceil() as i32 + 1
    } else { 1 }.max(1);
    let cols_max = if span_w > 0.0 && step_min_x > 0.0 && overlap_max < 1.0 {
        (span_w / step_min_x).floor() as i32 + 1
    } else { i32::MAX };
    let rows_min = if span_h > 0.0 && step_max_y > 0.0 {
        (span_h / step_max_y).ceil() as i32 + 1
    } else { 1 }.max(1);
    let rows_max = if span_h > 0.0 && step_min_y > 0.0 && overlap_max < 1.0 {
        (span_h / step_min_y).floor() as i32 + 1
    } else { i32::MAX };

    let actual_cols = target_cols.max(cols_min).min(cols_max).max(1);
    let actual_rows = target_rows.max(rows_min).min(rows_max).max(1);

    let step_x = if actual_cols > 1 {
        (span_w / (actual_cols - 1) as f64).round() as i32
    } else { img_w };
    let step_y = if actual_rows > 1 {
        (span_h / (actual_rows - 1) as f64).round() as i32
    } else { img_h };

    let mut drag_x = (step_x as f64 / consts.rate_x).round() as i32;
    let mut drag_y = (step_y as f64 / consts.rate_y).round() as i32;

    drag_x = drag_x.min(1280).min(mode_max_x);
    drag_y = drag_y.min(720).min(mode_max_y);

    let clamped = actual_rows != target_rows || actual_cols != target_cols;
    let overlap_x = if img_w > 0 { 1.0 - step_x as f64 / img_w_f } else { 0.0 };
    let overlap_y = if img_h > 0 { 1.0 - step_y as f64 / img_h_f } else { 0.0 };

    Some(DerivedFromBase {
        img_w, img_h, target_w, target_h,
        actual_rows, actual_cols,
        drag_x, drag_y,
        overlap_x, overlap_y,
        clamped,
    })
}

pub fn derive_from_target(
    client_w: i32,
    client_h: i32,
    scroll_mode: &str,
    target_w: i32,
    target_h: i32,
    overlap_min: f64,
    overlap_max: f64,
) -> Option<DerivedFromBase> {
    let _ = overlap_max;
    let consts = get_mode_constants(scroll_mode)?;
    let img_w = (client_w as f64 * consts.capture_region_x).round() as i32;
    let img_h = (client_h as f64 * consts.capture_region_y).round() as i32;

    if target_w < img_w || target_h < img_h {
        return None;
    }

    let img_w_f = img_w as f64;
    let img_h_f = img_h as f64;
    let span_w = (target_w - img_w) as f64;
    let span_h = (target_h - img_h) as f64;

    let (mode_max_x, mode_max_y) = get_drag_max(scroll_mode)?;
    let step_max_x = (mode_max_x as f64 * consts.rate_x * 1.005).min(img_w_f * (1.0 - overlap_min));
    let step_max_y = (mode_max_y as f64 * consts.rate_y * 1.005).min(img_h_f * (1.0 - overlap_min));

    let actual_cols = if span_w > 0.0 && step_max_x > 0.0 {
        ((span_w / step_max_x).ceil() as i32 + 1).max(2)
    } else { 1 };
    let actual_rows = if span_h > 0.0 && step_max_y > 0.0 {
        ((span_h / step_max_y).ceil() as i32 + 1).max(2)
    } else { 1 };

    let step_x = if actual_cols > 1 {
        (span_w / (actual_cols - 1) as f64).round() as i32
    } else { img_w };
    let step_y = if actual_rows > 1 {
        (span_h / (actual_rows - 1) as f64).round() as i32
    } else { img_h };

    let drag_x = ((step_x as f64 / consts.rate_x).round() as i32).min(1280).min(mode_max_x);
    let drag_y = ((step_y as f64 / consts.rate_y).round() as i32).min(720).min(mode_max_y);

    let overlap_x = if img_w > 0 { 1.0 - step_x as f64 / img_w_f } else { 0.0 };
    let overlap_y = if img_h > 0 { 1.0 - step_y as f64 / img_h_f } else { 0.0 };

    Some(DerivedFromBase {
        img_w, img_h, target_w, target_h,
        actual_rows, actual_cols,
        drag_x, drag_y,
        overlap_x, overlap_y,
        clamped: false,
    })
}

// ============ 0-8 次推导 ============

/// 9 个滚动次数的推导结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct AllCountsResult {
    /// 9 个次数的推导结果，索引 0-8 对应 scroll_count 0-8
    pub counts: Vec<DerivedFromBase>,
}

/// 从 count=0 的 target 推导所有 0-8 次的数据
pub fn derive_all_counts_from_base(
    client_w: i32,
    client_h: i32,
    target_w: i32,
    target_h: i32,
    overlap_min: f64,
    overlap_max: f64,
) -> Option<AllCountsResult> {
    let mut counts = Vec::with_capacity(9);

    let k_arr = COUNT_K;
    for i in 0..9 {
        let (kx, ky) = k_arr[i];
        let t_w = (target_w as f64 * kx).round() as i32;
        let t_h = (target_h as f64 * ky).round() as i32;
        let derived = derive_from_target(
            client_w, client_h,
            &format!("{}次", i),
            t_w, t_h,
            overlap_min, overlap_max,
        )?;
        counts.push(derived);
    }

    Some(AllCountsResult { counts })
}