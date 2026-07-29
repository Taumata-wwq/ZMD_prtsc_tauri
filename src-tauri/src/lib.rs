use std::sync::Arc;

use tauri::Manager;

mod commands;
mod error;
mod models;
mod services;

use commands::capture::CaptureState;
use services::auto_capture::AutoCapture;
use services::persistence::DbState;

/// 检查当前进程是否以管理员权限运行（供 main.rs 调用）
pub fn is_admin() -> bool {
    services::uac::is_admin()
}

/// 运行时提权：以管理员权限重启自身，旧进程等待新进程退出（供 main.rs 调用）
pub fn relaunch_as_admin_and_wait() -> Result<(), String> {
    services::uac::relaunch_as_admin_and_wait()
}

pub fn run() {
    // UAC 提权已在 main.rs 中通过 relaunch_as_admin_and_wait() 处理
    // 此处进程已是管理员权限（或用户拒绝 UAC 后以非管理员权限继续）

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();

            // 注：禁用 webview 右键菜单已在前端通过 contextmenu 事件阻止实现
            // （Tauri 2 的 WebviewWindow 运行时无 disable_context_menu 方法）

            // 初始化数据库（需要 app handle 获取 app_data_dir）
            // AppError 已实现 std::error::Error，可直接用 `?` 转换为 setup 要求的
            // `Result<(), Box<dyn std::error::Error + Send + Sync>>`
            let db = DbState::init(&handle)?;
            app.manage(db);

            // 初始化 AutoCapture（全局 Arc 单例，跨 command 共享）
            let auto_capture = Arc::new(AutoCapture::new(handle));
            app.manage(auto_capture);

            // 初始化 CaptureState（跟踪 current / total / region_name）
            app.manage(CaptureState::default());

            // 注册全局热键 F3（开始/停止合并为 F3，失败不阻止启动，仅打印警告）
            if let Err(e) = services::hotkey::register_hotkeys(&app.handle()) {
                eprintln!("警告：全局热键注册失败: {}", e);
                // 不阻止启动，继续执行
            }

            // 恢复窗口状态（位置/大小/置顶）
            if let Err(e) = services::window_state::restore_window_state(&app.handle()) {
                eprintln!("警告：窗口状态恢复失败: {}", e);
            }

            // 初始化窗口事件监听（moved/resized 防抖保存）
            if let Err(e) = services::window_state::init_window_state_listener(&app.handle()) {
                eprintln!("警告：窗口事件监听初始化失败: {}", e);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // capture
            commands::capture::start_capture,
            commands::capture::stop_capture,
            commands::capture::get_capture_status,
            commands::capture::get_preview_image,
            commands::capture::get_preview_path,
            // image
            commands::image::export_image,
            // config
            commands::config::list_regions,
            commands::config::get_region,
            commands::config::upsert_region,
            commands::config::delete_region,
            commands::config::list_scroll_modes,
            commands::config::derive_region_params,
            commands::config::derive_from_base,
            commands::config::derive_from_target,
            commands::config::derive_all_counts,
            commands::config::derive_region_from_target,
            // settings
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::get_all_settings,
            commands::settings::set_many_settings,
            // history
            commands::history::list_sessions,
            commands::history::clear_history,
            // window
            commands::window::find_game_window,
            commands::window::activate_game_window,
            commands::window::load_window_state,
            commands::window::save_window_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}