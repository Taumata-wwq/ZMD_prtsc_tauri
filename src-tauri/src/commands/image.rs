use std::path::Path;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::models::session::CropBox;
use crate::services::persistence::DbState;
use crate::services::resolve_unique_path;
use crate::services::stitcher;

/// 导出图像：解码源文件 → 裁剪 → 编码 → 写入唯一路径，并回写 session
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn export_image(
    source_path: String,
    crop: Option<CropBox>,
    format: String,
    quality: u8,
    output_path: String,
    session_id: Option<i64>,
    crop_box_json: Option<String>,
    db: State<'_, DbState>,
) -> AppResult<String> {
    let output_for_block = output_path.clone();
    let actual_path = tokio::task::spawn_blocking(move || -> AppResult<String> {
        if !Path::new(&source_path).exists() {
            return Err(AppError::new(
                format!("源文件不存在: {}", source_path),
                "SOURCE_NOT_FOUND",
            ));
        }
        let data = std::fs::read(&source_path).map_err(|e| AppError::new(
            format!("读取源文件失败: {}", e),
            "IO_ERROR",
        ))?;

        let dynamic = image::load_from_memory(&data)
            .map_err(|e| AppError::new(format!("解码图像失败: {}", e), "IMAGE_DECODE_ERROR"))?;
        let mut rgba = dynamic.to_rgba8();

        if let Some(c) = crop {
            rgba = stitcher::crop_image(&rgba, (c.x, c.y, c.w, c.h))?;
        }

        let format_upper = format.to_uppercase();
        let bytes: Vec<u8> = match format_upper.as_str() {
            "JPG" | "JPEG" => stitcher::encode_jpeg(&rgba, quality)?,
            "PNG" => stitcher::encode_png(&rgba, false)?,
            _ => return Err(AppError::new(
                format!("不支持的图像格式: {}", format),
                "INVALID_FORMAT",
            )),
        };

        let path_obj = Path::new(&output_for_block);
        if let Some(parent) = path_obj.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::new(
                format!("创建输出目录失败: {}", e),
                "IO_ERROR",
            ))?;
        }
        let unique_path = resolve_unique_path(path_obj);
        std::fs::write(&unique_path, &bytes).map_err(|e| AppError::new(
            format!("写入文件失败: {}", e),
            "IO_ERROR",
        ))?;
        Ok(unique_path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| AppError::new(format!("导出任务执行失败: {}", e), "TASK_JOIN_ERROR"))??;

    if let Some(sid) = session_id {
        let crop_box_str = crop_box_json.as_deref();
        if let Err(e) = db.update_session_paths(sid, None, Some(&actual_path), crop_box_str) {
            eprintln!("警告：回写 session 路径失败: {}", e);
        }
    }

    Ok(actual_path)
}
