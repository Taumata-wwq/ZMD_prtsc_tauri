#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Write;

fn main() {
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("zmd_debug.log")));

    log_line(&log_path, "[main] 启动");
    log_line(&log_path, &format!("[main] current_dir={:?}", std::env::current_dir()));
    log_line(&log_path, &format!("[main] exe={:?}", std::env::current_exe()));

    // dev 模式下 cargo run 会创建控制台窗口，启动后隐藏
    hide_console_window();

    // debug 跳过 relaunch（会丢失 dev server 上下文）；release 作为 manifest 嵌入失败的 fallback
    #[cfg(not(debug_assertions))]
    {
        if !zmd_prtsc_tauri_lib::is_admin() {
            log_line(&log_path, "[main] 非管理员权限，尝试 relaunch 提权...");
            match zmd_prtsc_tauri_lib::relaunch_as_admin_and_wait() {
                Ok(()) => {
                    log_line(&log_path, "[main] relaunch 成功，当前进程退出");
                    return;
                }
                Err(e) => {
                    log_line(&log_path, &format!("[main] UAC 提权失败: {}，以非管理员模式继续", e));
                }
            }
        } else {
            log_line(&log_path, "[main] 已是管理员权限");
        }
    }
    #[cfg(debug_assertions)]
    {
        if !zmd_prtsc_tauri_lib::is_admin() {
            log_line(&log_path, "[main] debug 构建：非管理员权限，跳过 relaunch（如需管理员权限请以管理员身份运行终端）");
        } else {
            log_line(&log_path, "[main] debug 构建：已是管理员权限");
        }
    }

    log_line(&log_path, "[main] 调用 run()...");

    let result = std::panic::catch_unwind(|| {
        zmd_prtsc_tauri_lib::run();
    });

    match result {
        Ok(()) => {
            log_line(&log_path, "[main] run() 正常返回");
        }
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "<unknown panic>".to_string()
            };
            log_line(&log_path, &format!("[main] PANIC: {}", msg));
            log_line(&log_path, "[main] 按回车键退出...");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
    }

    log_line(&log_path, "[main] 进程退出");
}

// 写入日志行到文件（追加）和 stderr
fn log_line(log_path: &Option<std::path::PathBuf>, msg: &str) {
    eprintln!("{}", msg);
    if let Some(path) = log_path {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{}", msg);
        }
    }
}

// 隐藏 dev 模式下 cargo run 创建的控制台窗口
fn hide_console_window() {
    use windows::Win32::System::Console::GetConsoleWindow;
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

    unsafe {
        let hwnd = GetConsoleWindow();
        if !hwnd.is_invalid() {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}
