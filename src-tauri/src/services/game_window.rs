use crate::error::{AppError, AppResult};
use std::sync::Mutex;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClientRect, GetWindowTextLengthW,
    GetWindowTextW, GetWindowRect, IsWindow, IsWindowVisible, SetForegroundWindow,
    ShowWindow, SW_RESTORE, SW_SHOW,
};

/// 窗口信息（游戏窗口检测返回）
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub title: String,
    pub hwnd: isize,
    pub left: i32,
    pub top: i32,
}

/// 客户区矩形（屏幕坐标）
#[derive(Debug, Clone, Copy)]
pub struct ClientRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

/// 缓存的游戏窗口句柄
static CACHED_HWND: Mutex<Option<isize>> = Mutex::new(None);

/// 查找标题包含 "Endfield" 的游戏窗口
pub fn find_endfield_window() -> AppResult<Option<WindowInfo>> {
    // 先尝试缓存的句柄
    if let Some(cached) = *CACHED_HWND.lock().unwrap() {
        if let Some(info) = validate_and_get_info(cached)? {
            return Ok(Some(info));
        }
    }

    // 枚举所有顶层窗口查找 Endfield
    let found: Option<HWND> = enumerate_windows(|hwnd| {
        if let Some(title) = get_window_title(hwnd) {
            if title.to_lowercase().contains("endfield") {
                return true;  // 找到，停止枚举
            }
        }
        false
    })?;

    if let Some(hwnd) = found {
        let hwnd_ptr = hwnd.0 as isize;
        *CACHED_HWND.lock().unwrap() = Some(hwnd_ptr);
        let info = validate_and_get_info(hwnd_ptr)?;
        Ok(info)
    } else {
        Ok(None)
    }
}

/// 验证句柄有效性并获取窗口信息
fn validate_and_get_info(hwnd_raw: isize) -> AppResult<Option<WindowInfo>> {
    let hwnd = HWND(hwnd_raw as *mut _);
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        *CACHED_HWND.lock().unwrap() = None;
        return Ok(None);
    }
    let title = get_window_title(hwnd).unwrap_or_default();
    let rect = get_window_rect(hwnd)?;
    Ok(Some(WindowInfo {
        title,
        hwnd: hwnd_raw,
        left: rect.left,
        top: rect.top,
    }))
}

/// 获取窗口标题
fn get_window_title(hwnd: HWND) -> Option<String> {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return None;
        }
        let mut buffer = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buffer);
        if copied == 0 {
            return None;
        }
        String::from_utf16(&buffer[..copied as usize]).ok()
    }
}

/// 获取窗口矩形（屏幕坐标）
fn get_window_rect(hwnd: HWND) -> AppResult<RECT> {
    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut rect)?;
    }
    Ok(rect)
}

/// 枚举所有顶层窗口，回调返回 true 表示找到目标（停止枚举）
///
/// `EnumWindows` 回调须为 C ABI 函数指针，无法直接捕获闭包。
/// 这里通过 LPARAM 传递堆上 `EnumContext` 裸指针：调用方 `Box::into_raw` 取指针，
/// `enum_proc` 借用上下文调用 callback，结束后 `Box::from_raw` 恢复所有权。
fn enumerate_windows<F>(callback: F) -> AppResult<Option<HWND>>
where
    F: FnMut(HWND) -> bool,
{
    /// 枚举上下文：携带回调与找到的窗口句柄
    struct EnumContext<F> {
        callback: F,
        found: Option<HWND>,
    }

    /// EnumWindows 回调（C ABI，泛型单态化）
    /// 通过 LPARAM 恢复 `EnumContext` 裸指针并调用 callback，true 表示停止枚举。
    extern "system" fn enum_proc<F>(hwnd: HWND, lparam: LPARAM) -> BOOL
    where
        F: FnMut(HWND) -> bool,
    {
        let ctx_ptr = lparam.0 as *mut EnumContext<F>;
        // 安全：上下文由调用方 Box 持有，EnumWindows 同步返回前指针有效
        let ctx = unsafe { &mut *ctx_ptr };
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return BOOL(1); // 跳过不可见窗口，继续枚举
        }
        if (ctx.callback)(hwnd) {
            ctx.found = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }

    let ctx_box = Box::new(EnumContext {
        callback,
        found: None,
    });
    let ctx_ptr = Box::into_raw(ctx_box);

    let result = unsafe { EnumWindows(Some(enum_proc::<F>), LPARAM(ctx_ptr as isize)) };

    // 无论 EnumWindows 是否成功都必须回收 Box，避免内存泄漏
    let ctx = unsafe { Box::from_raw(ctx_ptr) };

    // 优先检查 ctx.found：回调返回 FALSE 主动停止枚举时 EnumWindows 会返回 Err
    if let Some(hwnd) = ctx.found {
        return Ok(Some(hwnd));
    }

    if let Err(err) = result {
        return Err(AppError::new(
            format!("枚举窗口失败: {}", err),
            "ENUM_WINDOWS_FAILED",
        ));
    }

    Ok(None)
}

/// 获取客户区矩形（屏幕坐标）
pub fn get_client_rect(hwnd_raw: isize) -> AppResult<ClientRect> {
    let hwnd = HWND(hwnd_raw as *mut _);
    let mut client_rect = RECT::default();
    unsafe {
        GetClientRect(hwnd, &mut client_rect)?;
    }
    let client_w = client_rect.right - client_rect.left;
    let client_h = client_rect.bottom - client_rect.top;

    let mut top_left = windows::Win32::Foundation::POINT { x: 0, y: 0 };
    unsafe {
        if !ClientToScreen(hwnd, &mut top_left).as_bool() {
            return Err(AppError::new(
                "ClientToScreen 转换失败（窗口可能已销毁）",
                "CLIENT_TO_SCREEN_FAILED",
            ));
        }
    }

    Ok(ClientRect {
        left: top_left.x,
        top: top_left.y,
        width: client_w,
        height: client_h,
    })
}

/// 激活窗口（不置顶）：恢复最小化 + 激活到前台让游戏获取焦点，不用 HWND_TOPMOST 避免遮挡其他窗口
pub fn activate_window() -> AppResult<()> {
    let hwnd_raw = (*CACHED_HWND.lock().unwrap())
        .ok_or_else(|| AppError::new("未找到游戏窗口句柄", "WINDOW_FIND_FAILED"))?;
    let hwnd = HWND(hwnd_raw as *mut _);

    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        *CACHED_HWND.lock().unwrap() = None;
        return Err(AppError::new(
            "游戏窗口句柄无效",
            "WINDOW_HANDLE_INVALID",
        ));
    }

    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }

    // 等待 500ms 让窗口完成激活（避免过长延迟）
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(())
}
