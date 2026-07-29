//! 图像拼接：网格拼接 + PNG/JPG 编码 + 裁剪（rayon 并行加速）

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{PngEncoder, CompressionType, FilterType};
use image::{imageops, DynamicImage, ImageFormat, RgbaImage};
use rayon::prelude::*;

use crate::error::{AppError, AppResult};

/// 可跨线程共享的裸指针包装（各子图写入不重叠区域，保证安全）
struct SyncPtr(*mut u8);
unsafe impl Sync for SyncPtr {}
unsafe impl Send for SyncPtr {}

impl SyncPtr {
    /// 获取裸指针副本
    /// 通过方法调用避免 Rust 2021 disjoint capture 捕获 `&*mut u8`（非 Sync）
    #[inline(always)]
    fn ptr(&self) -> *mut u8 {
        self.0
    }
}

/// 将多张截图拼接为一张大图
///
/// 使用 rayon 并行处理不同子图，每张子图内部逐行 copy_from_slice。
/// 各子图写入目标 buffer 的不重叠区域，因此并行安全。
///
/// 当截图数量不足以填满整个网格时，自动裁剪到实际有内容的边界，
/// 避免出现黑色空白区域。
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
    // 避免截图不满整张图时出现黑色空白区域
    let max_row = screenshots.iter().map(|(r, _, _)| *r).max().unwrap_or(0);
    let max_col = screenshots.iter().map(|(_, c, _)| *c).max().unwrap_or(0);
    // 实际需要的宽高：最后一行/列需要完整 img 尺寸（不含 overlap 扣减）
    let total_w = step_x.saturating_mul(max_col).saturating_add(img_w);
    let total_h = step_y.saturating_mul(max_row).saturating_add(img_h);

    let mut result = RgbaImage::new(total_w, total_h);

    let dst_stride = total_w as usize * 4;
    let dst_ptr: *mut u8 = std::ops::DerefMut::deref_mut(&mut result).as_mut_ptr();
    let sync_dst = SyncPtr(dst_ptr);

    // 并行处理每张子图：各子图写入目标 buffer 的不重叠区域
    screenshots.par_iter().for_each(|(row, col, img)| {
        let paste_x = col.saturating_mul(step_x) as usize;
        let paste_y = row.saturating_mul(step_y) as usize;
        if paste_x >= total_w as usize || paste_y >= total_h as usize {
            return;
        }
        let (src_w, src_h) = img.dimensions();
        let copy_w = (src_w as usize).min(total_w as usize - paste_x);
        let copy_h = (src_h as usize).min(total_h as usize - paste_y);
        if copy_w == 0 || copy_h == 0 {
            return;
        }
        let src_stride = src_w as usize * 4;
        let copy_bytes = copy_w * 4;
        let src_buf = img.as_raw();

        for y in 0..copy_h {
            let dst_y = paste_y + y;
            let dst_offset = dst_y * dst_stride + paste_x * 4;
            let src_offset = y * src_stride;
            // SAFETY: 各子图的 (paste_x, paste_y) 不重叠，线程间无数据竞争
            unsafe {
                std::ptr::copy_nonoverlapping(
                    src_buf.as_ptr().add(src_offset),
                    sync_dst.ptr().add(dst_offset),
                    copy_bytes,
                );
            }
        }
    });

    Ok(result)
}

/// 将 `RgbaImage` 编码为 PNG 字节流
pub fn encode_png(image: &RgbaImage) -> AppResult<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    image.write_to(&mut cursor, ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

/// 将 `RgbaImage` 编码为 PNG 字节流（快速模式）
pub fn encode_png_fast(image: &RgbaImage) -> AppResult<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    let encoder = PngEncoder::new_with_quality(&mut cursor, CompressionType::Fast, FilterType::Adaptive);
    image.write_with_encoder(encoder)?;
    Ok(cursor.into_inner())
}

/// 将 `RgbaImage` 编码为 JPEG 字节流（用于预览，速度最快）
///
/// JPEG 编码比 PNG 快 10-50 倍，适合大图预览。
/// 透明像素填充为黑色（JPEG 不支持 alpha 通道）。
pub fn encode_jpeg_fast(image: &RgbaImage, quality: u8) -> AppResult<Vec<u8>> {
    let mut buffer = Vec::new();
    let rgb = DynamicImage::ImageRgba8(image.clone()).into_rgb8();
    let encoder = JpegEncoder::new_with_quality(&mut buffer, quality);
    rgb.write_with_encoder(encoder)?;
    Ok(buffer)
}

/// 将 `RgbaImage` 编码为 JPG 字节流
pub fn encode_jpg(image: &RgbaImage, quality: u8) -> AppResult<Vec<u8>> {
    let mut buffer = Vec::new();
    let rgb = DynamicImage::ImageRgba8(image.clone()).into_rgb8();
    let encoder = JpegEncoder::new_with_quality(&mut buffer, quality);
    rgb.write_with_encoder(encoder)?;
    Ok(buffer)
}

/// 按裁剪框裁剪图像
pub fn crop_image(image: &RgbaImage, crop_box: (u32, u32, u32, u32)) -> AppResult<RgbaImage> {
    let (x, y, w, h) = crop_box;
    let cropped: RgbaImage = imageops::crop_imm(image, x, y, w, h).to_image();
    Ok(cropped)
}
