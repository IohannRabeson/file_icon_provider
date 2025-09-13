use std::{fmt::Display, path::Path};

/// Represents an icon with its dimensions and pixel data.
pub struct Icon {
    /// The width of the icon in pixels.
    pub width: u32,
    /// The height of the icon in pixels.
    pub height: u32,
    /// The pixel data of the icon in RGBA format.
    pub pixels: Vec<u8>,
}

/// Represents an error
#[derive(Debug)]
pub enum Error {
    /// Retrieving the icon failed
    Failed,
    /// The path does not exist
    PathDoesNotExist,
    /// The desired icon size is null
    NullIconSize,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Failed => {
                write!(f, "Failed to get icon")
            }
            Error::PathDoesNotExist => {
                write!(f, "Path does not exist")
            }
            Error::NullIconSize => {
                write!(f, "Null icon size")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Retrieves the icon for a given file.
///
/// # Parameters
/// * `path` - A file path for which the icon is to be retrieved.
/// * `size` - Desired icon size, must be greater than 0.
/// # Returns
/// * `Ok(Icon)` - If the icon is successfully retrieved.
/// * `Err(Error)` - If the icon could not be retrieved.
///
/// # Example
/// ```
/// use file_icon_provider::get_file_icon;
///
/// if let Ok(icon) = get_file_icon("path/to/file", 64) {
///     println!("Icon dimensions: {}x{}", icon.width, icon.height);
/// } else {
///     println!("Failed to retrieve the icon.");
/// }
/// ```
pub fn get_file_icon(path: impl AsRef<Path>, size: u16) -> Result<Icon, Error> {
    // For consistency: on MacOS if the path does not exist None is returned
    // but on Windows a default icon is returned.
    if !path.as_ref().exists() {
        return Err(Error::PathDoesNotExist);
    }

    if size == 0 {
        return Err(Error::NullIconSize);
    }

    implementation::get_file_icon(path, size).ok_or(Error::Failed)
}

pub type Provider = implementation::Provider;

mod implementation {
    #[cfg(target_os = "macos")]
    mod macos;

    #[cfg(target_os = "macos")]
    pub(crate) use macos::get_file_icon;

    #[cfg(target_os = "macos")]
    pub(crate) use macos::Provider;

    #[cfg(target_os = "windows")]
    #[allow(non_upper_case_globals)]
    pub(crate) fn get_file_icon(path: impl AsRef<Path>, size: u16) -> Option<Icon> {
        use scopeguard::defer;
        use windows::{
            Win32::{
                Foundation::SIZE,
                Graphics::{
                    Gdi::DeleteObject,
                    Imaging::{
                        CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGRA,
                        GUID_WICPixelFormat32bppRGBA, IWICImagingFactory, WICBitmapUseAlpha,
                        WICRect,
                    },
                },
                System::Com::{CLSCTX_ALL, CoCreateInstance, CoInitialize, CoUninitialize},
                UI::Shell::{
                    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY,
                    SIIGBF_SCALEUP,
                },
            },
            core::HSTRING,
        };

        unsafe {
            CoInitialize(None).ok().ok()?;
        }

        defer!(unsafe {
            CoUninitialize();
        });

        let path = HSTRING::from(path.as_ref());
        let image_factory: IShellItemImageFactory =
            unsafe { SHCreateItemFromParsingName(&path, None) }.ok()?;
        let bitmap_size = SIZE {
            cx: size as i32,
            cy: size as i32,
        };
        let bitmap = unsafe {
            image_factory
                .GetImage(bitmap_size, SIIGBF_ICONONLY | SIIGBF_SCALEUP)
                .ok()?
        };
        defer!(unsafe {
            let _ = DeleteObject(bitmap);
        });

        let imaging_factory: IWICImagingFactory =
            unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_ALL).ok()? };
        let bitmap = unsafe {
            imaging_factory
                .CreateBitmapFromHBITMAP(bitmap, None, WICBitmapUseAlpha)
                .ok()?
        };
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

                unsafe {
                    bitmap
                        .CopyPixels(&source_rectangle, size as u32 * 4, &mut pixels)
                        .ok()?
                };

                if pixel_format == GUID_WICPixelFormat32bppBGRA {
                    for chunk in pixels.chunks_exact_mut(4) {
                        chunk.swap(0, 2);
                    }
                }

                pixels
            }
            _ => panic!(
                "Unsupported pixel format: {:?}\nPlease create an issue: https://github.com/IohannRabeson/file_icon_provider/issues/new?title=Unsupported%20pixel%20format%20{:?}",
                pixel_format, pixel_format
            ),
        };

        Some(Icon {
            width: size as u32,
            height: size as u32,
            pixels,
        })
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn get_file_icon(path: impl AsRef<Path>, size: u16) -> Option<Icon> {
        use gio::{
            Cancellable, File, FileQueryInfoFlags,
            prelude::{Cast, FileExt},
        };
        use gtk::{IconLookupFlags, IconTheme, prelude::IconThemeExt};

        if !gtk::is_initialized() {
            gtk::init().ok()?;
        }

        let file = File::for_path(path);
        let file_info = file
            .query_info("*", FileQueryInfoFlags::NONE, None::<&Cancellable>)
            .ok()?;
        let content_type = file_info.content_type()?;
        let icon = gio::functions::content_type_get_icon(&content_type);

        if let Some(icon) = icon.dynamic_cast_ref::<gio::ThemedIcon>() {
            let icon_theme = IconTheme::default()?;

            for name in icon.names() {
                if let Some(pixbuf) = icon_theme
                    .load_icon(&name, size as i32, IconLookupFlags::empty())
                    .ok()
                    .flatten()
                {
                    return Some(Icon {
                        width: pixbuf.width() as u32,
                        height: pixbuf.height() as u32,
                        pixels: pixbuf.read_pixel_bytes().to_vec(),
                    });
                }
            }

            None
        } else {
            panic!("Unsupported icon type");
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
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

        assert!(get_file_icon(program_file_path, 32).is_ok());
    }

    #[test]
    fn test_not_existing_file() {
        assert!(get_file_icon("NOT EXISTING", 32).is_err());
    }

    #[test]
    fn test_null_icon_size() {
        let program_file_path = std::env::args().next().expect("get program path");
        let program_file_path = PathBuf::from(&program_file_path);

        assert!(get_file_icon(program_file_path, 0).is_err());
    }
}
