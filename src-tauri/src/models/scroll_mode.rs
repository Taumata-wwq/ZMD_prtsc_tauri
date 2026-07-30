use serde::{Deserialize, Serialize};

/// 滚动次数配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollMode {
    pub id: Option<i64>,
    pub name: String,
    pub scroll_count: i32,
    pub is_default: bool,
}

/// 默认滚动次数列表（0-8 次）
/// 0 次 = 距地面最近/视野最小；8 次 = 距地面最远/视野最大
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

/// 截图区域比例（所有次数统一）
pub(crate) const CAPTURE_REGION_X: f64 = 0.626;
pub(crate) const CAPTURE_REGION_Y: f64 = 0.648;

/// 大地图专用截图区域比例
pub(crate) const CAPTURE_REGION_X_LARGE_MAP: f64 = 0.378;
pub(crate) const CAPTURE_REGION_Y_LARGE_MAP: f64 = 0.388;

// 视图移动倍率 rate（step = drag × rate）
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

// 拖拽距离最大值约束（基于 1280×720 最小支持画面）
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

// target 比例常量 k（k[count] = target_count / target_0）
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

/// 从 scroll_mode 字符串解析 scroll_count（"0次"→0, "5次"→5, ...）
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

// 根据滚动次数获取基准截图区域常量，未知次数返回 None
fn get_mode_constants(scroll_mode: &str) -> Option<ModeConstants> {
    let count = parse_scroll_count(scroll_mode)?;
    let rate = COUNT_RATES.get(count as usize)?;
    Some(ModeConstants {
        capture_region_x: CAPTURE_REGION_X,
        capture_region_y: CAPTURE_REGION_Y,
        rate_x: *rate,
        rate_y: *rate,
    })
}

fn get_drag_max_by_count(count: i32) -> Option<(i32, i32)> {
    COUNT_DRAG_MAX.get(count as usize).copied()
}

fn get_drag_max(scroll_mode: &str) -> Option<(i32, i32)> {
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

// 基础截图尺寸参数
struct BaseParams {
    img_w: i32,
    img_h: i32,
}

fn compute_base_params(
    capture_region_x: f64,
    capture_region_y: f64,
    client_w: i32,
    client_h: i32,
) -> BaseParams {
    let img_w = (client_w as f64 * capture_region_x).round() as i32;
    let img_h = (client_h as f64 * capture_region_y).round() as i32;
    BaseParams { img_w, img_h }
}

fn compute_step_max(mode_max: i32, rate: f64, img_f: f64, overlap_min: f64) -> f64 {
    // 0.5% 余量避免边界精度误差导致最后一列/行无法完整覆盖
    (mode_max as f64 * rate * 1.005).min(img_f * (1.0 - overlap_min))
}

fn compute_step_from_count(actual_count: i32, span: f64, img: i32) -> i32 {
    if actual_count > 1 {
        (span / (actual_count - 1) as f64).round() as i32
    } else {
        img
    }
}

// 计算 drag 并约束到 hard_max（最小支持画面 1280×720）与 mode_max
fn compute_drag(step: i32, rate: f64, hard_max: i32, mode_max: i32) -> i32 {
    ((step as f64 / rate).round() as i32).min(hard_max).min(mode_max)
}

fn compute_overlap_from_step(step: i32, img: i32, img_f: f64) -> f64 {
    if img > 0 {
        1.0 - step as f64 / img_f
    } else {
        0.0
    }
}

pub fn derive_from_target(
    client_w: i32,
    client_h: i32,
    scroll_mode: &str,
    target_w: i32,
    target_h: i32,
    overlap_min: f64,
) -> Option<DerivedFromBase> {
    let consts = get_mode_constants(scroll_mode)?;
    let base = compute_base_params(consts.capture_region_x, consts.capture_region_y, client_w, client_h);
    let img_w = base.img_w;
    let img_h = base.img_h;

    if target_w < img_w || target_h < img_h {
        return None;
    }

    let img_w_f = img_w as f64;
    let img_h_f = img_h as f64;
    let span_w = (target_w - img_w) as f64;
    let span_h = (target_h - img_h) as f64;

    let (mode_max_x, mode_max_y) = get_drag_max(scroll_mode)?;
    let step_max_x = compute_step_max(mode_max_x, consts.rate_x, img_w_f, overlap_min);
    let step_max_y = compute_step_max(mode_max_y, consts.rate_y, img_h_f, overlap_min);

    let actual_cols = if span_w > 0.0 && step_max_x > 0.0 {
        ((span_w / step_max_x).ceil() as i32 + 1).max(2)
    } else { 1 };
    let actual_rows = if span_h > 0.0 && step_max_y > 0.0 {
        ((span_h / step_max_y).ceil() as i32 + 1).max(2)
    } else { 1 };

    let step_x = compute_step_from_count(actual_cols, span_w, img_w);
    let step_y = compute_step_from_count(actual_rows, span_h, img_h);

    let drag_x = compute_drag(step_x, consts.rate_x, 1280, mode_max_x);
    let drag_y = compute_drag(step_y, consts.rate_y, 720, mode_max_y);

    let overlap_x = compute_overlap_from_step(step_x, img_w, img_w_f);
    let overlap_y = compute_overlap_from_step(step_y, img_h, img_h_f);

    Some(DerivedFromBase {
        img_w, img_h, target_w, target_h,
        actual_rows, actual_cols,
        drag_x, drag_y,
        overlap_x, overlap_y,
        clamped: false,
    })
}

/// 9 个滚动次数的推导结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct AllCountsResult {
    /// 索引 0-8 对应 scroll_count 0-8
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
    let _ = overlap_max;
    let mut counts = Vec::with_capacity(9);

    let k_arr = COUNT_K;
    for (i, &(kx, ky)) in k_arr.iter().enumerate() {
        let t_w = (target_w as f64 * kx).round() as i32;
        let t_h = (target_h as f64 * ky).round() as i32;
        let derived = derive_from_target(
            client_w, client_h,
            &format!("{}次", i),
            t_w, t_h,
            overlap_min,
        )?;
        counts.push(derived);
    }

    Some(AllCountsResult { counts })
}
