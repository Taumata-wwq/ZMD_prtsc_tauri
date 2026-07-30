//! 图像拼接：网格拼接 + PNG/JPG 编码 + 裁剪（rayon 并行加速）

use std::collections::BTreeMap;
use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{PngEncoder, CompressionType, FilterType};
use image::{imageops, DynamicImage, ImageFormat, RgbaImage};
use rayon::prelude::*;

use crate::error::{AppError, AppResult};

/// 将多张截图拼接为一张大图
///
/// 两阶段策略：先并行横向拼接每行（缓存友好的连续内存写入），再串行纵向粘贴各行。
/// 相比单阶段直接写入超大 buffer，行内拼接缓存命中率更高，对大网格（如 15×9）性能显著提升。
/// 截图不足以填满网格时自动裁剪到实际内容边界。
pub fn stitch_images(
    screenshots: &[(u32, u32, RgbaImage)],
    grid: (u32, u32),
    overlap: (f64, f64),
) -> AppResult<RgbaImage> {
    if screenshots.is_empty() {
        return Err(AppError::new("没有截图可拼接", "STITCH_EMPTY"));
    }

    let (rows, cols) = grid;
    if rows == 0 || cols == 0 {
        return Err(AppError::new("网格行列数必须大于 0", "STITCH_INVALID_GRID"));
    }

    let (img_w, img_h) = screenshots[0].2.dimensions();
    if img_w == 0 || img_h == 0 {
        return Err(AppError::new("子图尺寸不能为 0", "STITCH_INVALID_SIZE"));
    }

    let (overlap_x, overlap_y) = overlap;
    let overlap_px_x = (img_w as f64 * overlap_x) as u32;
    let overlap_px_y = (img_h as f64 * overlap_y) as u32;
    let step_x = img_w.saturating_sub(overlap_px_x);
    let step_y = img_h.saturating_sub(overlap_px_y);

    // 计算实际有截图的最大行/列，裁剪到实际内容边界
    let max_row = screenshots.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
    let max_col = screenshots.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
    let total_w = step_x.saturating_mul(max_col).saturating_add(img_w);
    let total_h = step_y.saturating_mul(max_row).saturating_add(img_h);

    // 按行分组：row -> Vec<(col, &RgbaImage)>，BTreeMap 保证行顺序
    let mut rows_map: BTreeMap<u32, Vec<(u32, &RgbaImage)>> = BTreeMap::new();
    for (r, c, img) in screenshots.iter() {
        rows_map.entry(*r).or_default().push((*c, img));
    }

    // 阶段1：并行横向拼接每行（宽度 = step_x * row_max_col + img_w，高度 = img_h）
    let row_images: Vec<(u32, RgbaImage)> = rows_map
        .into_par_iter()
        .map(|(row, mut cols_vec)| {
            cols_vec.sort_by_key(|(c, _)| *c);
            let row_max_col = cols_vec.last().map(|(c, _)| *c).unwrap_or(0);
            let row_w = step_x.saturating_mul(row_max_col).saturating_add(img_w);
            let mut row_img = RgbaImage::new(row_w, img_h);

            for (col, img) in cols_vec {
                let paste_x = col.saturating_mul(step_x);
                if paste_x >= row_w {
                    continue;
                }
                let (src_w, src_h) = img.dimensions();
                let copy_w = src_w.min(row_w.saturating_sub(paste_x));
                let copy_h = src_h.min(img_h);
                if copy_w == 0 || copy_h == 0 {
                    continue;
                }
                paste_subimage_horizontal(&mut row_img, img, paste_x, copy_w, copy_h);
            }

            (row, row_img)
        })
        .collect();

    // 阶段2：串行纵向粘贴各行（行数远少于子图数，大块连续复制，缓存友好）
    let mut result = RgbaImage::new(total_w, total_h);
    let dst_stride = total_w as usize * 4;
    let dst_ptr: *mut u8 = std::ops::DerefMut::deref_mut(&mut result).as_mut_ptr();

    for (row, row_img) in row_images {
        let paste_y = row.saturating_mul(step_y);
        if paste_y >= total_h {
            continue;
        }
        let (row_w, row_h) = row_img.dimensions();
        let copy_w = row_w.min(total_w);
        let copy_h = row_h.min(total_h.saturating_sub(paste_y));
        if copy_w == 0 || copy_h == 0 {
            continue;
        }

        let src_stride = row_w as usize * 4;
        let copy_bytes = copy_w as usize * 4;
        let src_buf = row_img.as_raw();
        let src_ptr = src_buf.as_ptr();

        // 逐行复制：从 row_img 复制到 result 的 (0, paste_y) 位置
        for y in 0..copy_h as usize {
            let dst_offset = (paste_y as usize + y) * dst_stride;
            let src_offset = y * src_stride;
            // SAFETY: src（row_img）与 dst（result）是独立的 buffer，区域不重叠
            unsafe {
                std::ptr::copy_nonoverlapping(
                    src_ptr.add(src_offset),
                    dst_ptr.add(dst_offset),
                    copy_bytes,
                );
            }
        }
    }

    Ok(result)
}

/// 将子图横向粘贴到行图像的指定 x 位置（高度相同，逐行复制连续内存块）
fn paste_subimage_horizontal(
    row_img: &mut RgbaImage,
    img: &RgbaImage,
    paste_x: u32,
    copy_w: u32,
    copy_h: u32,
) {
    let row_w = row_img.width();
    let dst_stride = row_w as usize * 4;
    let src_stride = img.width() as usize * 4;
    let copy_bytes = copy_w as usize * 4;
    let src_buf = img.as_raw();
    let dst_buf = std::ops::DerefMut::deref_mut(row_img);
    let dst_ptr = dst_buf.as_mut_ptr();
    let src_ptr = src_buf.as_ptr();

    for y in 0..copy_h as usize {
        let dst_offset = y * dst_stride + paste_x as usize * 4;
        let src_offset = y * src_stride;
        // SAFETY: 各子图在行图像中的 paste_x 不重叠
        unsafe {
            std::ptr::copy_nonoverlapping(
                src_ptr.add(src_offset),
                dst_ptr.add(dst_offset),
                copy_bytes,
            );
        }
    }
}

/// 将 `RgbaImage` 编码为 PNG 字节流
/// `fast=true` 用 Fast 压缩 + Adaptive 滤波（截图场景）；`false` 用默认设置。
pub fn encode_png(image: &RgbaImage, fast: bool) -> AppResult<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    if fast {
        let encoder = PngEncoder::new_with_quality(&mut cursor, CompressionType::Fast, FilterType::Adaptive);
        image.write_with_encoder(encoder)?;
    } else {
        image.write_to(&mut cursor, ImageFormat::Png)?;
    }
    Ok(cursor.into_inner())
}

/// 将 `RgbaImage` 编码为 JPEG 字节流（透明像素填充为黑色，JPEG 不支持 alpha）
pub fn encode_jpeg(image: &RgbaImage, quality: u8) -> AppResult<Vec<u8>> {
    let mut buffer = Vec::new();
    let rgb = DynamicImage::ImageRgba8(image.clone()).into_rgb8();
    let encoder = JpegEncoder::new_with_quality(&mut buffer, quality);
    rgb.write_with_encoder(encoder)?;
    Ok(buffer)
}

/// 生成缩略图字节流（JPEG，按比例缩放到指定最大宽度，Lanczos3 滤波）
pub fn generate_thumbnail(image: &RgbaImage, max_width: u32) -> AppResult<Vec<u8>> {
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 {
        return Err(AppError::new("无效的图像尺寸", "INVALID_IMAGE"));
    }
    let new_w = w.min(max_width);
    let new_h = ((h as f64) * (new_w as f64 / w as f64)).round() as u32;
    let thumb = imageops::resize(image, new_w, new_h, imageops::FilterType::Lanczos3);
    encode_jpeg(&thumb, 85)
}

/// 按裁剪框裁剪图像
pub fn crop_image(image: &RgbaImage, crop_box: (u32, u32, u32, u32)) -> AppResult<RgbaImage> {
    let (x, y, w, h) = crop_box;
    let cropped: RgbaImage = imageops::crop_imm(image, x, y, w, h).to_image();
    Ok(cropped)
}
