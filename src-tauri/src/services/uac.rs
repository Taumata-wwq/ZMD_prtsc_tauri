//! UAC 提权服务
//!
//! 通过 `ShellExecuteExW("runas")` 运行时提权，GNU 工具链下 manifest 方案不可用。

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{GetCurrentProcess, WaitForSingleObject};
use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_NOCLOSEPROCESS};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// 检查当前进程是否以管理员权限运行
///
/// 通过 OpenProcessToken + GetTokenInformation(TokenElevation) 判断。
/// 失败时返回 false（保守判断为非管理员）。
pub fn is_admin() -> bool {
    use windows::Win32::System::Threading::OpenProcessToken;

    unsafe {
        let mut token: HANDLE = HANDLE::default();
        let process = GetCurrentProcess();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret_len = 0u32;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );

        let _ = CloseHandle(token);

        result.is_ok() && elevation.TokenIsElevated != 0
    }
}

/// 运行时提权：以管理员权限重启自身，旧进程等待新进程退出
///
/// 流程：
/// 1. 获取当前 exe 路径
/// 2. 用 `ShellExecuteExW("runas")` 启动新管理员进程
/// 3. 旧进程等待新进程退出（`WaitForSingleObject`）
/// 4. 新进程退出后，旧进程返回（随后 main 返回，进程退出）
///
/// 在 dev 模式下，旧进程等待期间 cargo run 不会退出，
/// vite dev server 保持运行，新管理员进程可正常连接。
///
/// # 返回
/// - `Ok(())`：新进程已退出，旧进程应立即退出
/// - `Err(String)`：提权失败（用户拒绝 UAC 或其他错误），旧进程以非管理员权限继续
pub fn relaunch_as_admin_and_wait() -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("获取当前 exe 路径失败: {}", e))?;

    // 传递当前工作目录给新进程，避免新进程在 System32 下启动导致找不到资源
    let cwd = std::env::current_dir()
        .map_err(|e| format!("获取当前工作目录失败: {}", e))?;

    let exe_wide = to_wide(&exe_path.to_string_lossy());
    let verb_wide = to_wide("runas");
    let cwd_wide = to_wide(&cwd.to_string_lossy());

    eprintln!("[uac] 提权重启: exe={}, cwd={}", exe_path.display(), cwd.display());

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb_wide.as_ptr()),
        lpFile: PCWSTR(exe_wide.as_ptr()),
        lpParameters: PCWSTR::null(),
        lpDirectory: PCWSTR(cwd_wide.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    unsafe {
        // ShellExecuteExW 返回 Result<(), Error>，失败时用户可能拒绝了 UAC
        let result = ShellExecuteExW(&mut info);
        if result.is_err() {
            return Err("ShellExecuteExW 失败（用户可能拒绝了 UAC 提示）".to_string());
        }

        // 等待新进程退出
        // dev 模式下：cargo run 不会退出，vite dev server 保持运行
        // 新管理员进程连接 vite 直到用户关闭它
        if !info.hProcess.is_invalid() {
            eprintln!("[uac] 等待新管理员进程退出...");
            let _ = WaitForSingleObject(info.hProcess, u32::MAX);
            let _ = CloseHandle(info.hProcess);
            eprintln!("[uac] 新管理员进程已退出");
        }
    }

    Ok(())
}

/// 将字符串转换为以 null 结尾的 UTF-16 宽字符向量
fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
