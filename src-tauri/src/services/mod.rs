pub mod auto_capture;
pub mod capture_pipeline;
pub mod game_window;
pub mod hotkey;
pub mod input;
pub mod input_win;
pub mod persistence;
pub mod screenshot;
pub mod seed_data;
pub mod shared;
pub mod stitcher;
pub mod uac;
pub mod window_state;

pub use shared::{emit_log, resolve_unique_path};
