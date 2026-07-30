use std::sync::Arc;

use tauri::Manager;

mod commands;
mod error;
mod models;
mod services;

use services::auto_capture::AutoCapture;
use services::persistence::DbState;

pub use services::uac::{is_admin, relaunch_as_admin_and_wait};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();

            let db = DbState::init(&handle)?;
            app.manage(db);

            // AutoCapture 全局单例
            let auto_capture = Arc::new(AutoCapture::new(handle));
            app.manage(auto_capture);

            // F3 热键注册失败不阻止启动
            if let Err(e) = services::hotkey::register_hotkeys(app.handle()) {
                eprintln!("警告：全局热键注册失败: {}", e);
            }

            if let Err(e) = services::window_state::restore_window_state(app.handle()) {
                eprintln!("警告：窗口状态恢复失败: {}", e);
            }

            if let Err(e) = services::window_state::init_window_state_listener(app.handle()) {
                eprintln!("警告：窗口事件监听初始化失败: {}", e);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // capture
            commands::capture::start_capture,
            commands::capture::stop_capture,
            commands::capture::get_preview_image,
            commands::capture::get_preview_path,
            commands::capture::set_preview_path,
            // image
            commands::image::export_image,
            // config
            commands::config::list_regions,
            commands::config::upsert_region,
            commands::config::delete_region,
            commands::config::list_scroll_modes,
            commands::config::derive_all_counts,
            commands::config::derive_region_from_target,
            // settings
            commands::settings::set_setting,
            commands::settings::get_all_settings,
            commands::settings::set_many_settings,
            commands::settings::reset_data,
            // history
            commands::history::list_sessions,
            commands::history::delete_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}