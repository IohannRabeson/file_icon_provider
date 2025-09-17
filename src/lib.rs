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

/// Provider is interesting if you request a lot of icons with a fixed size.  
/// It allocates internal buffers once and reuse them.  
/// It caches icons reducing the CPU and memory usage.  
pub struct Provider<T: Clone> {
    implementation: implementation::Provider<T>,
}

impl<T> Provider<T>
where
    T: Clone,
{
    pub fn new(icon_size: u16, converter: fn(Icon) -> T) -> Result<Self, Error> {
        if icon_size == 0 {
            return Err(Error::NullIconSize);
        }

        Ok(Self {
            implementation: implementation::Provider::new(icon_size, converter)
                .ok_or(Error::Failed)?,
        })
    }

    pub fn get_file_icon(&self, path: impl AsRef<Path>) -> Result<T, Error> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(Error::PathDoesNotExist);
        }

        self.implementation.get_file_icon(path).ok_or(Error::Failed)
    }
}

mod implementation {
    #[cfg(target_os = "macos")]
    mod macos;

    #[cfg(target_os = "macos")]
    pub(crate) use macos::get_file_icon;

    #[cfg(target_os = "macos")]
    pub(crate) use macos::Provider;

    #[cfg(target_os = "windows")]
    mod windows;

    #[cfg(target_os = "windows")]
    pub(crate) use windows::get_file_icon;

    #[cfg(target_os = "windows")]
    pub(crate) use windows::Provider;

    #[cfg(target_os = "linux")]
    mod linux;

    #[cfg(target_os = "linux")]
    pub(crate) use linux::get_file_icon;

    #[cfg(target_os = "linux")]
    pub(crate) use linux::Provider;

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    pub(crate) fn get_file_icon(path: impl AsRef<Path>, size: u16) -> Option<Icon> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{Icon, Provider, get_file_icon};
    use std::rc::Rc;

    #[test]
    fn test_get_file_icon() {
        let file_path = locate_cargo_manifest::locate_manifest().expect("locate Cargo.toml");

        assert!(get_file_icon(file_path, 32).is_ok());
    }

    #[test]
    fn test_not_existing_file() {
        assert!(get_file_icon("NOT EXISTING", 32).is_err());
    }

    #[test]
    fn test_null_icon_size() {
        let file_path = locate_cargo_manifest::locate_manifest().expect("locate Cargo.toml");

        assert!(get_file_icon(file_path, 0).is_err());
    }

    #[test]
    fn test_get_file_icon_provider() {
        let file_path = locate_cargo_manifest::locate_manifest().expect("locate Cargo.toml");
        let provider =
            Provider::<Rc<Icon>>::new(32, |icon| Rc::new(icon)).expect("create provider");

        assert!(provider.get_file_icon(file_path).is_ok());
    }

    #[test]
    fn test_mixed_usages() {
        let file_path = locate_cargo_manifest::locate_manifest().expect("locate Cargo.toml");
        let provider =
            Provider::<Rc<Icon>>::new(32, |icon| Rc::new(icon)).expect("create provider");

        let result = provider.get_file_icon(&file_path);

        if let Err(error) = &result {
            println!("error: {}", error);
        }

        assert!(result.is_ok());
        assert!(get_file_icon(&file_path, 32).is_ok());
    }
}
