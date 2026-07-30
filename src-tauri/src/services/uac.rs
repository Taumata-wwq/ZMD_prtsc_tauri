//! UAC 提权：运行时 relaunch 为管理员
//!
//! 作为 app.manifest 嵌入失败时的 fallback。
//! manifest 嵌入成功时，进程启动即为管理员，is_admin() 返回 true，relaunch 不执行。

use crate::error::{AppError, AppResult};
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcessToken, WaitForSingleObject, INFINITE,
};
use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_NOCLOSEPROCESS};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// 检查当前进程是否以管理员权限运行
pub fn is_admin() -> bool {
    is_admin_inner().unwrap_or(false)
}

/// 通过 OpenProcessToken + GetTokenInformation(TokenElevation) 检查提权状态
fn is_admin_inner() -> AppResult<bool> {
    unsafe {
        let mut token = windows::Win32::Foundation::HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)?;
        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret_len = 0u32;
        GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut std::ffi::c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        )?;
        Ok(elevation.TokenIsElevated != 0)
    }
}

/// 以管理员权限重新启动当前 exe，并等待其退出后退出当前进程
///
/// 使用 ShellExecuteExW with verb="runas" 触发 UAC 提权提示。
/// 提权成功后等待新进程退出，再返回 Ok 由调用方退出当前进程。
pub fn relaunch_as_admin_and_wait() -> AppResult<()> {
    let exe = std::env::current_exe()
        .map_err(|e| AppError::new(format!("无法获取当前 exe 路径: {}", e), "UAC_EXE_PATH"))?;

    let verb = to_wide("runas");
    let file = to_wide_os(exe.as_os_str());

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    let hprocess = unsafe {
        ShellExecuteExW(&mut info)?;
        info.hProcess
    };

    // 等待提权后的进程退出（hProcess 为 null 时表示未获取句柄，跳过等待）
    if !hprocess.0.is_null() {
        unsafe {
            let _ = WaitForSingleObject(hprocess, INFINITE);
        }
    }

    Ok(())
}

/// 将 &str 转为以 null 结尾的 UTF-16 宽字符
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 将 OsStr 转为以 null 结尾的 UTF-16 宽字符
fn to_wide_os(s: &std::ffi::OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}
