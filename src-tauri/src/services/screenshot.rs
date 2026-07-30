//! 屏幕截图：Win32 GDI BitBlt + GetDIBits

use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap,
    CreateCompatibleDC, DeleteDC, DeleteObject, DIB_RGB_COLORS, GetDC, GetDIBits,
    HBITMAP, HDC, HGDIOBJ, ReleaseDC, RGBQUAD, SelectObject, SRCCOPY,
};

use crate::error::{AppError, AppResult};
use crate::services::game_window;

/// 截取屏幕指定矩形区域（屏幕坐标），返回 RGBA 像素数据
fn capture_screen_rect(rect: (i32, i32, i32, i32)) -> AppResult<image::RgbaImage> {
    let (left, top, width, height) = rect;
    if width <= 0 || height <= 0 {
        return Err(AppError::new(
            format!("无效的截图区域: ({}, {}, {}, {})", left, top, width, height),
            "INVALID_RECT",
        ));
    }

    unsafe {
        // 1. 获取屏幕 DC（None 等价于 NULL HWND，表示整个虚拟屏幕）
        let screen_dc = GetDC(None);
        if screen_dc.is_invalid() {
            return Err(AppError::new("GetDC 获取屏幕 DC 失败", "GETDC_FAILED"));
        }

        // 2. 创建兼容内存 DC
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        if mem_dc.is_invalid() {
            let _ = ReleaseDC(None, screen_dc);
            return Err(AppError::new("CreateCompatibleDC 失败", "CREATE_DC_FAILED"));
        }

        // 3. 创建兼容位图（必须基于 screen_dc，否则颜色位数不匹配）
        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.is_invalid() {
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(None, screen_dc);
            return Err(AppError::new(
                "CreateCompatibleBitmap 失败",
                "CREATE_BITMAP_FAILED",
            ));
        }

        // 4. 选入位图到内存 DC，保存旧对象以便后续还原
        let old_obj = SelectObject(mem_dc, HGDIOBJ(bitmap.0));

        // 5. BitBlt 从屏幕 DC 复制到内存 DC
        if let Err(e) = BitBlt(
            mem_dc,
            0,
            0,
            width,
            height,
            Some(screen_dc),
            left,
            top,
            SRCCOPY,
        ) {
            cleanup(old_obj, mem_dc, bitmap, screen_dc);
            return Err(AppError::new(format!("BitBlt 失败: {}", e), "BITBLT_FAILED"));
        }

        // 6. 准备 BITMAPINFO 用于 GetDIBits
        //    - biHeight 为负值表示 top-down DIB，避免行序翻转
        //    - biBitCount = 32 表示每像素 32 位（BGRA）
        //    - BI_RGB 是 BI_COMPRESSION(0u32) 枚举常量，需用 `.0` 取出 u32 值
        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // 负值表示 top-down
                biPlanes: 1,
                biBitCount: 32, // 32-bit BGRA
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default()],
        };

        // 7. GetDIBits 获取像素数据（返回成功扫描的行数，0 表示失败）
        let pixel_count = (width as usize) * (height as usize);
        let mut pixels: Vec<u8> = vec![0u8; pixel_count * 4];
        let scanned = GetDIBits(
            mem_dc,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        );

        // 清理 GDI 资源（无论 GetDIBits 是否成功都要清理）
        cleanup(old_obj, mem_dc, bitmap, screen_dc);

        if scanned == 0 {
            return Err(AppError::new(
                "GetDIBits 获取像素数据失败",
                "GETDIBITS_FAILED",
            ));
        }

        // 8. 像素格式转换：BGRA -> RGBA（交换 B 与 R 通道，Alpha 保持 255）
        for chunk in pixels.chunks_mut(4) {
            chunk.swap(0, 2);
        }

        // 9. 构造 RgbaImage 并返回
        image::RgbaImage::from_raw(width as u32, height as u32, pixels).ok_or_else(|| {
            AppError::new("构造 RgbaImage 失败", "IMAGE_CONSTRUCTION_FAILED")
        })
    }
}

/// 释放 GDI 资源：还原旧对象 → 删除位图 → 删除内存 DC → 释放屏幕 DC
unsafe fn cleanup(old_obj: HGDIOBJ, mem_dc: HDC, bitmap: HBITMAP, screen_dc: HDC) {
    let _ = SelectObject(mem_dc, old_obj);
    let _ = DeleteObject(bitmap.into());
    let _ = DeleteDC(mem_dc);
    let _ = ReleaseDC(None, screen_dc);
}

/// 截取游戏窗口客户区中心区域，根据 capture_region_x/y 比例和 offset_y 计算截图矩形
pub fn capture_center_region(
    hwnd: isize,
    region_x_ratio: f64,
    region_y_ratio: f64,
    offset_y_ratio: f64,
) -> AppResult<image::RgbaImage> {
    // 1. 获取客户区屏幕坐标
    let client_rect = game_window::get_client_rect(hwnd)?;

    let client_left = client_rect.left;
    let client_top = client_rect.top;
    let client_w = client_rect.width;
    let client_h = client_rect.height;

    if client_w <= 0 || client_h <= 0 {
        return Err(AppError::new(
            format!("客户区尺寸无效: {}x{}", client_w, client_h),
            "INVALID_CLIENT_RECT",
        ));
    }

    // 2. 计算截图区域尺寸
    let region_w = (client_w as f64 * region_x_ratio).round() as i32;
    let region_h = (client_h as f64 * region_y_ratio).round() as i32;
    let offset_y = (client_h as f64 * offset_y_ratio).round() as i32;

    if region_w <= 0 || region_h <= 0 {
        return Err(AppError::new(
            format!(
                "计算出的截图区域尺寸无效: {}x{} (比例 x={}, y={})",
                region_w, region_h, region_x_ratio, region_y_ratio
            ),
            "INVALID_REGION_SIZE",
        ));
    }

    // 3. 计算截图区域左上角（屏幕坐标）
    let x = client_left + (client_w - region_w) / 2;
    let y = client_top + (client_h - region_h) / 2 + offset_y;

    // 4. 调用底层截图函数
    capture_screen_rect((x, y, region_w, region_h))
}