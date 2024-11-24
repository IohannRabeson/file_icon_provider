use std::path::Path;

/// Represents an icon with its dimensions and pixel data.
pub struct Icon {
    /// The width of the icon in pixels.
    pub width: u32,
    /// The height of the icon in pixels.
    pub height: u32,
    /// The pixel data of the icon in RGBA format.
    pub pixels: Vec<u8>,
}

/// Retrieves the icon for a given file.
///
/// # Parameters
/// * `path` - A file path for which the icon is to be retrieved.
///
/// # Returns
/// * `Some(Icon)` - If the icon is successfully retrieved.
/// * `None` - If the icon could not be retrieved.
///
/// # Example
/// ```
/// use file_icon_provider::get_file_icon;
///
/// if let Some(icon) = get_file_icon("path/to/file") {
///     println!("Icon dimensions: {}x{}", icon.width, icon.height);
/// } else {
///     println!("Failed to retrieve the icon.");
/// }
/// ```
pub fn get_file_icon(path: impl AsRef<Path>) -> Option<Icon> {
    implementation::get_file_icon(path)
}

mod implementation {
    use super::*;

    #[cfg(target_os = "macos")]
    pub fn get_file_icon(path: impl AsRef<Path>) -> Option<Icon> {
        use objc2::ClassType;
        use objc2_app_kit::{
            NSBitmapImageRep, NSCompositingOperation, NSGraphicsContext, NSWorkspace,
        };
        use objc2_foundation::{CGPoint, CGRect, NSString};

        let path = path.as_ref().canonicalize().ok()?;
        let file_path = NSString::from_str(path.to_str()?);
        let color_space_name = NSString::from_str("NSDeviceRGBColorSpace");

        let shared_workspace = unsafe { NSWorkspace::sharedWorkspace() };
        let image = unsafe { shared_workspace.iconForFile(&file_path) };
        let image_size = unsafe { image.size() };
        let image_width = image_size.width as isize;
        let image_height = image_size.height as isize;

        if image_width < 1 || image_height < 1 {
            return None;
        }

        let pixels = unsafe {
            let bitmap_representation = NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            image_width,
            image_height,
            8,
            4,
            true,
            false,
            &color_space_name,
            image_width * 4,
            32,
        )?;
            let context =
                NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap_representation)?;

            context.saveGraphicsState();

            NSGraphicsContext::setCurrentContext(Some(&context));
            image.drawAtPoint_fromRect_operation_fraction(
                CGPoint::ZERO,
                CGRect::new(CGPoint::ZERO, image_size),
                NSCompositingOperation::Copy,
                1.0,
            );
            context.flushGraphics();
            context.restoreGraphicsState();

            std::slice::from_raw_parts(
                bitmap_representation.bitmapData(),
                bitmap_representation.bytesPerPlane() as usize,
            )
            .to_vec() 
        };

        Some(Icon {
            width: image_width as u32,
            height: image_height as u32,
            pixels,
        })
    }

    #[cfg(target_os = "windows")]
    fn get_file_icon(path: impl AsRef<Path>) -> Option<Icon> {
        use scopeguard::defer;
        use std::ffi::c_void;
        use windows::{
            core::HSTRING,
            Win32::{
                Foundation::{HANDLE, HWND},
                Graphics::Gdi::{
                    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetDIBits,
                    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
                    HBRUSH,
                },
                Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
                UI::{
                    Shell::{
                        SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON,
                        SHGFI_USEFILEATTRIBUTES,
                    },
                    WindowsAndMessaging::{DrawIconEx, GetIconInfo, DI_NORMAL, ICONINFO},
                },
            },
        };

        let file_path = HSTRING::from(path.as_ref());
        let info = &mut SHFILEINFOW::default();

        unsafe {
            SHGetFileInfoW(
                &file_path,
                FILE_ATTRIBUTE_NORMAL,
                Some(info as *mut SHFILEINFOW),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_USEFILEATTRIBUTES | SHGFI_SMALLICON,
            );
        }

        let mut icon_info = ICONINFO::default();

        unsafe {
            GetIconInfo(info.hIcon, &mut icon_info as *mut ICONINFO).ok()?;
            // Release ICONINFO crap immediatly as we don't need it so we are sure it can't leak (we are using the operator ?).
            // See Remark in https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-geticoninfo
            let _ = DeleteObject(icon_info.hbmColor);
            let _ = DeleteObject(icon_info.hbmMask);
        }

        assert!(icon_info.fIcon.as_bool());

        let width = (icon_info.xHotspot * 2) as i32;
        let height = (icon_info.yHotspot * 2) as i32;
        let bytes_count = (width * height) as u32 * 4;

        let rendering_bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biBitCount: 32,
                biPlanes: 1,
                biCompression: BI_RGB.0,
                biWidth: width,
                biHeight: height,
                biSizeImage: bytes_count,
                ..Default::default()
            },
            ..Default::default()
        };

        let hdc = unsafe {
            let screen_device = GetDC(HWND::default());
            let hdc = CreateCompatibleDC(screen_device);
            ReleaseDC(HWND::default(), screen_device);
            hdc
        };

        defer!(unsafe {
            let _ = DeleteDC(hdc);
        });

        let rendering_bitmap = unsafe {
            CreateDIBSection(
                hdc,
                &rendering_bitmap_info as *const BITMAPINFO,
                DIB_RGB_COLORS,
                std::ptr::null_mut(),
                HANDLE::default(),
                0,
            )
            .ok()?
        };

        defer!(unsafe {
            let _ = DeleteObject(rendering_bitmap);
        });

        let previous_hdc = unsafe { SelectObject(hdc, rendering_bitmap) };

        defer!(unsafe {
            SelectObject(hdc, previous_hdc);
        });

        unsafe {
            DrawIconEx(
                hdc,
                0,
                0,
                info.hIcon,
                width,
                height,
                0,
                HBRUSH::default(),
                DI_NORMAL,
            )
            .ok()?
        };

        let bitmap_info = &mut BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                // If biHeight is negative, the bitmap is a top-down DIB (Device Independent Bitmap) with the origin at the upper left corner.
                // See https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapinfoheader
                biHeight: -height,
                biSizeImage: bytes_count,
                biBitCount: 32,
                biPlanes: 1,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; bytes_count as usize];

        unsafe {
            if GetDIBits(
                hdc,
                rendering_bitmap,
                0,
                height as u32,
                Some(pixels.as_mut_ptr() as *mut c_void),
                bitmap_info as *mut BITMAPINFO,
                DIB_RGB_COLORS,
            ) == 0
            {
                return None;
            }
        }

        // GetDIBits gives BGRA colors but we want RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        Some(Icon {
            width: rendering_bitmap_info.bmiHeader.biWidth as u32,
            height: rendering_bitmap_info.bmiHeader.biHeight as u32,
            pixels,
        })
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    fn get_file_icon(path: impl AsRef<Path>) -> Option<Icon> {
        None
    }
}
