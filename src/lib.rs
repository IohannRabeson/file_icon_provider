use std::path::Path;

mod provider;

pub use provider::FileIconProvider;

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
/// if let Some(icon) = get_file_icon("path/to/file", 64) {
///     println!("Icon dimensions: {}x{}", icon.width, icon.height);
/// } else {
///     println!("Failed to retrieve the icon.");
/// }
/// ```
pub fn get_file_icon<'a>(path: impl AsRef<Path> + 'a, size: u16) -> Option<Icon> {
    // For consistency: on MacOS if the path does not exist None is returned
    // but on Windows a default icon is returned.
    if !path.as_ref().exists() {
        return None;
    }

    implementation::get_file_icon(path, size)
}

mod implementation {
    use super::*;

    #[cfg(target_os = "macos")]
    pub(crate) fn get_file_icon(path: impl AsRef<Path>, size: u16) -> Option<Icon> {
        use objc2::ClassType;
        use objc2_app_kit::{
            NSBitmapImageRep, NSCompositingOperation, NSGraphicsContext, NSWorkspace,
        };
        use objc2_foundation::{CGPoint, CGRect, CGSize, NSString};

        if size < 1 {
            return None;
        }

        let path = path.as_ref().canonicalize().ok()?;
        let file_path = NSString::from_str(path.to_str()?);
        let color_space_name = NSString::from_str("NSDeviceRGBColorSpace");
        let shared_workspace = unsafe { NSWorkspace::sharedWorkspace() };
        let image = unsafe { shared_workspace.iconForFile(&file_path) };
        let image_size = unsafe { image.size() };
        let desired_size = CGSize {
            width: size as f64,
            height: size as f64,
        };

        if image_size.width < 1.0 || image_size.height < 1.0 {
            return None;
        }

        let pixels = unsafe {
            let bitmap_representation = NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
                NSBitmapImageRep::alloc(),
                std::ptr::null_mut(),
                size as isize,
                size as isize,
                8,
                4,
                true,
                false,
                &color_space_name,
                size as isize * 4,
                32,
            )?;
            let context =
                NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap_representation)?;

            context.saveGraphicsState();

            NSGraphicsContext::setCurrentContext(Some(&context));

            image.setSize(desired_size);
            image.drawAtPoint_fromRect_operation_fraction(
                CGPoint::ZERO,
                CGRect::new(CGPoint::ZERO, desired_size),
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
            width: size as u32,
            height: size as u32,
            pixels,
        })
    }

    #[cfg(target_os = "windows")]
    #[allow(non_upper_case_globals)]
    pub(crate) fn get_file_icon(path: impl AsRef<Path>, size: u16) -> Option<Icon> {
        use scopeguard::defer;
        use windows::{core::HSTRING, Win32::{Foundation::SIZE, Graphics::{Gdi::DeleteObject, Imaging::{CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGRA, GUID_WICPixelFormat32bppRGBA, IWICImagingFactory, WICBitmapUseAlpha, WICRect}}, System::Com::{CoCreateInstance, CoInitialize, CoUninitialize, CLSCTX_ALL}, UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY, SIIGBF_SCALEUP}}};

        struct InitializationToken;

        impl Drop for InitializationToken {
            fn drop(&mut self) {
                unsafe { CoUninitialize(); }
            }
        }

        let _token = if unsafe{ CoInitialize(None) }.is_ok() {
            Some(InitializationToken)
        } else {
            None
        };
        
        let path = HSTRING::from(path.as_ref());
        let image_factory: IShellItemImageFactory = unsafe { SHCreateItemFromParsingName(&path, None) }.ok()?;
        let bitmap_size = SIZE {
            cx: size as i32,
            cy: size as i32,
        };
        let bitmap = unsafe { image_factory.GetImage(bitmap_size, SIIGBF_ICONONLY | SIIGBF_SCALEUP).ok()? };
        defer!(unsafe { let _ = DeleteObject(bitmap); });

        let imaging_factory: IWICImagingFactory = unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_ALL).ok()? };
        let bitmap = unsafe { imaging_factory.CreateBitmapFromHBITMAP(bitmap, None, WICBitmapUseAlpha).ok()? };
        let source_rectangle = WICRect {
            X: 0,
            Y: 0,
            Width: size as i32,
            Height: size as i32,
        };
        let pixel_format = unsafe { bitmap.GetPixelFormat().ok()? };
        let pixels = match pixel_format {
            GUID_WICPixelFormat32bppBGRA | GUID_WICPixelFormat32bppRGBA => {
                let mut pixels = vec![0u8; size as usize * size as usize * 4];

                unsafe { bitmap.CopyPixels(&source_rectangle, size as u32 * 4, &mut pixels).ok()? };

                if pixel_format == GUID_WICPixelFormat32bppBGRA {
                    for chunk in pixels.chunks_exact_mut(4) {
                        chunk.swap(0, 2);
                    }
                }

                pixels
            }
            _ => panic!("Unsupported pixel format: {:?}", pixel_format)
        };
        
        Some(Icon {
            width: size as u32,
            height: size as u32,
            pixels,
        })
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub(crate) fn get_file_icon(path: impl AsRef<Path>, size: u16) -> Option<Icon> {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::get_file_icon;

    #[test]
    fn test_get_file_icon() {
        let program_file_path = std::env::args().next().expect("get program path");
        let program_file_path = PathBuf::from(&program_file_path);

        println!("program_file_path: {}", program_file_path.display());
        assert!(get_file_icon(program_file_path, 32).is_some());
    }

    #[test]
    fn test_not_existing_file() {
        assert!(get_file_icon("NOT EXISTING", 32).is_none());
    }
}
