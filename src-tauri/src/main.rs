// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Write;

fn main() {
    // 初始化日志文件（诊断新管理员进程启动问题）
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("zmd_debug.log")));

    let admin = zmd_prtsc_tauri_lib::is_admin();
    log_line(&log_path, &format!("[main] 启动，is_admin={}", admin));
    log_line(&log_path, &format!("[main] current_dir={:?}", std::env::current_dir()));
    log_line(&log_path, &format!("[main] exe={:?}", std::env::current_exe()));

    // 隐藏控制台窗口（dev 模式下 cargo run 会显示控制台）
    hide_console_window();

    if !admin {
        log_line(&log_path, "[main] 非管理员权限，尝试提权...");
        match zmd_prtsc_tauri_lib::relaunch_as_admin_and_wait() {
            Ok(()) => {
                log_line(&log_path, "[main] 新管理员进程已退出，旧进程退出");
                return;
            }
            Err(e) => {
                log_line(&log_path, &format!("[main] 警告：管理员提权失败: {}", e));
                log_line(&log_path, "[main] 警告：游戏焦点下鼠标拖拽和 F3 热键可能失效");
            }
        }
    } else {
        log_line(&log_path, "[main] 已是管理员权限，直接启动 Tauri");
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

/// 将日志行写入文件（追加模式）和 stderr
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

/// 隐藏控制台窗口
///
/// 在 dev 模式下，cargo run 会创建控制台窗口。此函数在主窗口启动后隐藏它。
/// 使用 Win32 API: GetConsoleWindow() + ShowWindow(SW_HIDE)
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
