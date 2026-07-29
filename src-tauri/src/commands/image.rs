//! 图像导出命令：从磁盘路径读取源图，裁剪后按格式保存

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::models::session::CropBox;
use crate::services::persistence::DbState;
use crate::services::stitcher;

/// 从 source_path 读取源图，裁剪后按格式保存到 output_path
#[tauri::command]
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
    // 图像解码 / 裁剪 / 编码 / 文件写入均为同步阻塞操作，
    // 放到 spawn_blocking 中执行，避免阻塞 tokio runtime。
    let output_for_block = output_path.clone();
    tokio::task::spawn_blocking(move || -> AppResult<()> {
        // 1. 从磁盘读取源 PNG（避免前端传输大字节流）
        if !std::path::Path::new(&source_path).exists() {
            return Err(AppError::new(
                format!("源文件不存在: {}", source_path),
                "SOURCE_NOT_FOUND",
            ));
        }
        let data = std::fs::read(&source_path).map_err(|e| AppError::new(
            format!("读取源文件失败 {}: {}", source_path, e),
            "IO_ERROR",
        ))?;

        // 2. 解码 PNG bytes 为 DynamicImage，再转 RgbaImage（stitcher 接受 RgbaImage）
        let dynamic = image::load_from_memory(&data)
            .map_err(|e| AppError::new(format!("解码图像失败: {}", e), "IMAGE_DECODE_ERROR"))?;
        let mut rgba = dynamic.to_rgba8();

        // 3. 裁剪（若提供 crop）
        if let Some(c) = crop {
            rgba = stitcher::crop_image(&rgba, (c.x, c.y, c.w, c.h))?;
        }

        // 4. 根据 format 编码
        let format_upper = format.to_uppercase();
        let bytes: Vec<u8> = match format_upper.as_str() {
            "JPG" | "JPEG" => {
                // encode_jpg 内部使用 JpegEncoder，会自动将 RGBA 转为 RGB（丢弃 alpha）
                stitcher::encode_jpg(&rgba, quality)?
            }
            "PNG" => stitcher::encode_png(&rgba)?,
            _ => {
                return Err(AppError::new(
                    format!("不支持的图像格式: {}（仅支持 JPG / PNG）", format),
                    "INVALID_FORMAT",
                ));
            }
        };

        // 5. 写入文件（自动创建父目录，支持直接保存到 output_folder）
        let path_obj = std::path::Path::new(&output_for_block);
        if let Some(parent) = path_obj.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::new(
                format!("创建输出目录失败 {}: {}", parent.display(), e),
                "IO_ERROR",
            ))?;
        }
        std::fs::write(&output_for_block, &bytes).map_err(|e| AppError::new(
            format!("写入文件失败 {}: {}", path_obj.display(), e),
            "IO_ERROR",
        ))?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::new(format!("导出任务执行失败: {}", e), "TASK_JOIN_ERROR"))??;

    // 6. 回写 session 的 exported_path 和 crop_box（缺口3）
    //    session_id 为 None 时跳过（手动截图但未传 session_id 的场景）
    if let Some(sid) = session_id {
        let crop_box_str = crop_box_json.as_deref();
        if let Err(e) = db.update_session_paths(sid, None, Some(&output_path), crop_box_str) {
            // 回写失败不影响导出成功，仅打印警告
            eprintln!("警告：回写 session 路径失败: {}", e);
        }
    }

    // 7. 返回路径
    Ok(output_path)
}
