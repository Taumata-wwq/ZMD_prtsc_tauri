//! 输入模拟（SendInput + 中键拖拽）

use std::thread;
use std::time::Duration;

use rand::Rng;

use crate::services::input_win;

/// 随机延迟：base * random(0.9, 1.1)
pub fn rand_delay(base: f64) -> Duration {
    let mut rng = rand::thread_rng();
    let factor = rng.gen_range(0.9..1.1);
    Duration::from_secs_f64(base * factor)
}

/// ±30ms 抖动（模拟人类操作）
pub fn jitter_30ms() -> Duration {
    let mut rng = rand::thread_rng();
    let ms = rng.gen_range(0..=60);
    Duration::from_millis(ms)
}

/// 相机移动方向
#[derive(Debug, Clone, Copy)]
pub enum CameraDirection {
    Right,
    Left,
    Down,
}

/// 计算相机拖拽的起止坐标
pub fn calc_drag_coords(
    direction: CameraDirection,
    drag_distance: (i32, i32),
    client_left: i32,
    client_top: i32,
    client_h: i32,
    margin_bottom: i32,
    margin_left: i32,
) -> ((i32, i32), (i32, i32)) {
    let dx = drag_distance.0;
    let dy = drag_distance.1;
    // 横向拖拽：X 用 margin_left，Y 用 margin_bottom（距底边）
    let base_x = client_left + margin_left;
    let base_y_h = client_top + client_h - margin_bottom;
    // 纵向拖拽：X 用 margin_left，Y 用 margin_bottom（距顶边，用同值简化）
    let base_y_v = client_top + margin_bottom;
    let drag_x = client_left + dx + margin_left;
    let drag_y = client_top + dy + margin_bottom;

    match direction {
        CameraDirection::Right => ((drag_x, base_y_h), (base_x, base_y_h)),
        CameraDirection::Left => ((base_x, base_y_h), (drag_x, base_y_h)),
        CameraDirection::Down => ((base_x, drag_y), (base_x, base_y_v)),
    }
}

/// 执行相机移动（SendInput + 中键拖拽 + 随机稳定延迟）
pub fn move_camera(
    direction: CameraDirection,
    drag_distance: (i32, i32),
    client_left: i32,
    client_top: i32,
    client_h: i32,
    margin_bottom: i32,
    margin_left: i32,
    drag_duration: f64,
    stabilize_delay: f64,
) {
    let (start, end) = calc_drag_coords(
        direction,
        drag_distance,
        client_left,
        client_top,
        client_h,
        margin_bottom,
        margin_left,
    );
    input_win::drag_middle_sendinput(start.0, start.1, end.0, end.1, drag_duration);
    // 使用 stabilize_delay 随机延迟 + ±30ms 抖动
    thread::sleep(rand_delay(stabilize_delay) + jitter_30ms());
}

/// 执行滚轮滚动
pub fn do_scroll<F>(
    scroll_count: i32,
    center_x: i32,
    center_y: i32,
    mut is_running: F,
) where
    F: FnMut() -> bool,
{
    if scroll_count <= 0 {
        return;
    }
    input_win::sendinput_move_smooth(center_x, center_y, 0.1);
    thread::sleep(Duration::from_millis(100));
    for _ in 0..scroll_count {
        if !is_running() {
            return;
        }
        // 原 -120 滚动量过大，-3 仍然偏大，改为 -1（每次仅 1 格 WHEEL_DELTA）
        input_win::sendinput_scroll(-1);
        thread::sleep(Duration::from_millis(150));
    }
    thread::sleep(Duration::from_millis(500));
}

/// 拖拽刷新画面（滚轮滚动后的往返拖拽）
pub fn refresh_view(
    window_left: i32,
    window_top: i32,
    margin_bottom: i32,
    margin_left: i32,
) {
    let start_x = window_left + margin_left;
    let start_y = window_top + margin_bottom;
    let end_x = start_x + 300;
    let end_y = start_y + 300;
    // 往返拖拽：end → start → end（复刻 Python 原版的拖回去步骤）
    input_win::drag_middle_sendinput_roundtrip(end_x, end_y, start_x, start_y, 0.05);
    thread::sleep(Duration::from_millis(50));
}
