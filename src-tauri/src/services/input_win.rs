//! Windows 原生输入模拟（SendInput）
//!
//! 使用 Win32 SendInput API 实现鼠标移动、中键拖拽和滚轮滚动，
//! 通过 MOUSEEVENTF_VIRTUALDESK 标志确保多显示器下坐标正确。
//! 游戏仅响应中键拖拽。

use std::mem;
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_MOUSE, MOUSEINPUT, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL,
};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// WHEEL_DELTA 常量（Win32 标准值，每档滚轮 = 120）
const WHEEL_DELTA: u32 = 120;

/// 将屏幕坐标归一化为绝对坐标（0-65535），考虑多显示器虚拟桌面
fn normalize_absolute(x: i32, y: i32) -> (i32, i32) {
    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(1);
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(1);
    let norm_x = ((x as i64 * 65535) / screen_w as i64) as i32;
    let norm_y = ((y as i64 * 65535) / screen_h as i64) as i32;
    (norm_x, norm_y)
}

/// 通过 SendInput 移动鼠标到绝对坐标（使用虚拟桌面映射）
pub fn sendinput_move_abs(x: i32, y: i32) {
    let (nx, ny) = normalize_absolute(x, y);
    let mi = MOUSEINPUT {
        dx: nx,
        dy: ny,
        mouseData: 0,
        dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
        time: 0,
        dwExtraInfo: 0,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 { mi },
    };
    unsafe {
        let _ = SendInput(&[input], mem::size_of::<INPUT>() as i32);
    }
}

/// 按下鼠标中键
pub fn sendinput_middle_down() {
    let mi = MOUSEINPUT {
        dx: 0,
        dy: 0,
        mouseData: 0,
        dwFlags: MOUSEEVENTF_MIDDLEDOWN,
        time: 0,
        dwExtraInfo: 0,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 { mi },
    };
    unsafe {
        let _ = SendInput(&[input], mem::size_of::<INPUT>() as i32);
    }
}

/// 松开鼠标中键
pub fn sendinput_middle_up() {
    let mi = MOUSEINPUT {
        dx: 0,
        dy: 0,
        mouseData: 0,
        dwFlags: MOUSEEVENTF_MIDDLEUP,
        time: 0,
        dwExtraInfo: 0,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 { mi },
    };
    unsafe {
        let _ = SendInput(&[input], mem::size_of::<INPUT>() as i32);
    }
}

/// 平滑插值移动鼠标到目标位置
///
/// 在 (start_x, start_y) → (end_x, end_y) 之间按 16ms 间隔（约 60Hz）生成
/// 中间点，逐点调用 `sendinput_move_abs`，模拟真实鼠标移动轨迹。
fn interpolate_move_abs(start_x: i32, start_y: i32, end_x: i32, end_y: i32, duration_secs: f64) {
    if duration_secs <= 0.0 {
        sendinput_move_abs(end_x, end_y);
        return;
    }

    const STEP_MS: u64 = 16;
    const MIN_STEPS: u32 = 5;

    let total_us = (duration_secs * 1_000_000.0) as u64;
    let step_us = STEP_MS * 1000;
    let steps = ((total_us / step_us.max(1)).max(MIN_STEPS as u64)) as u32;

    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let x = (start_x as f64 + (end_x as f64 - start_x as f64) * t).round() as i32;
        let y = (start_y as f64 + (end_y as f64 - start_y as f64) * t).round() as i32;
        sendinput_move_abs(x, y);
        thread::sleep(Duration::from_millis(STEP_MS));
    }
}

/// SendInput + 中键拖拽
///
/// 使用 MOUSEEVENTF_VIRTUALDESK 标志确保多显示器下坐标正确。
pub fn drag_middle_sendinput(
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    drag_duration: f64,
) {
    // 1. 移动到起点
    sendinput_move_abs(start_x, start_y);
    thread::sleep(Duration::from_millis(30));

    // 2. 按下中键
    sendinput_middle_down();
    thread::sleep(Duration::from_millis(30));

    // 3. 平滑插值移动到终点
    interpolate_move_abs(start_x, start_y, end_x, end_y, drag_duration);

    // 4. 松开中键
    thread::sleep(Duration::from_millis(30));
    sendinput_middle_up();
}

/// SendInput + 中键往返拖拽（用于滚轮后刷新画面）
///
/// 复刻 Python 原版 do_scroll 末尾的拖拽逻辑：
/// 1. 移动到起点（终点位置 end）
/// 2. 按下中键
/// 3. 拖到 start（起点位置）
/// 4. 拖回 end（终点位置）— 拖回去步骤
/// 5. 松开中键
pub fn drag_middle_sendinput_roundtrip(
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    drag_duration: f64,
) {
    // 1. 移动到起点（对应 Python 的 end 位置）
    sendinput_move_abs(start_x, start_y);
    thread::sleep(Duration::from_millis(30));

    // 2. 按下中键
    sendinput_middle_down();
    thread::sleep(Duration::from_millis(30));

    // 3. 拖到终点（对应 Python 的 start 位置）
    interpolate_move_abs(start_x, start_y, end_x, end_y, drag_duration);

    // 4. 拖回起点（对应 Python 再 moveTo end_x, end_y）
    interpolate_move_abs(end_x, end_y, start_x, start_y, drag_duration);

    // 5. 松开中键
    thread::sleep(Duration::from_millis(30));
    sendinput_middle_up();
}

/// 获取当前鼠标光标位置（屏幕坐标）
pub fn get_cursor_pos() -> (i32, i32) {
    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        let _ = GetCursorPos(&mut point);
    }
    (point.x, point.y)
}

/// 平滑移动鼠标到目标位置（从当前位置线性插值）
///
/// 通过 GetCursorPos 获取起点，按 16ms 间隔（约 60Hz）插值移动到终点，
/// 模拟真实鼠标移动轨迹。
pub fn sendinput_move_smooth(x: i32, y: i32, duration_secs: f64) {
    let (cur_x, cur_y) = get_cursor_pos();
    interpolate_move_abs(cur_x, cur_y, x, y, duration_secs);
}

/// 发送滚轮事件（pyautogui 兼容语义：负数向下，正数向上）
///
/// 每次发送 1 个 WHEEL_DELTA（120）的滚动量，逐个发送以兼容游戏输入处理。
pub fn sendinput_scroll(amount: i32) {
    let direction = if amount < 0 { -1 } else { 1 };
    let count = amount.abs();
    for _ in 0..count {
        let mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: (direction as i32 * WHEEL_DELTA as i32) as u32,
            dwFlags: MOUSEEVENTF_WHEEL,
            time: 0,
            dwExtraInfo: 0,
        };
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 { mi },
        };
        unsafe {
            let _ = SendInput(&[input], mem::size_of::<INPUT>() as i32);
        }
    }
}
