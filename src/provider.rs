use std::{
    cell::RefCell,
    collections::{
        btree_map::Entry::{Occupied, Vacant},
        BTreeMap,
    },
    ffi::OsString,
    path::Path,
};

use crate::{get_file_icon, Icon};

/// This provider caches icons retrieved using [get_file_icon]
/// into a dictionary where keys are file extensions.  
///
/// The type T must be the final representation of the icon.
/// You must specify how the [Icon] returned by [get_file_icon] is
/// converted into T when creating [FileIconProvider].
pub struct FileIconProvider<T: Clone> {
    cache: RefCell<BTreeMap<(u16, OsString), T>>,
    convert: fn(Icon) -> T,
}

impl<T: Clone> FileIconProvider<T> {
    /// Create a new FileIconProvider specifying how to convert [Icon] into T.
    /// ```
    /// use file_icon_provider::{FileIconProvider, Icon};
    /// use image::{DynamicImage, RgbaImage};
    ///
    /// fn convert_icon(icon: Icon) -> DynamicImage {
    ///     DynamicImage::ImageRgba8(RgbaImage::from_raw(icon.width, icon.height, icon.pixels).unwrap())
    /// }
    /// let provider = FileIconProvider::new(convert_icon);
    /// ```
    pub fn new(convert: fn(Icon) -> T) -> Self {
        Self {
            cache: RefCell::new(BTreeMap::new()),
            convert,
        }
    }

    /// Retrieves the icon for a given file.
    pub fn icon(&self, path: impl AsRef<Path>, size: u16) -> Option<T> {
        let path = path.as_ref();
        let get_icon = |path| get_file_icon(path, size).map(self.convert);

        match path.extension() {
            Some(extension) => match self.cache.borrow_mut().entry((size, extension.to_owned())) {
                Vacant(vacant_entry) => Some(vacant_entry.insert(get_icon(path)?).clone()),
                Occupied(occupied_entry) => Some(occupied_entry.get().clone()),
            },
            // No extension then no caching.
            None => get_icon(path),
        }
    }

    /// Clear the cache.
    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }
}
