use std::{
    cell::RefCell, collections::BTreeMap, ffi::OsStr, os::unix::fs::PermissionsExt, path::Path,
};

use crate::Icon;

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
    let icon = icon.dynamic_cast_ref::<gio::ThemedIcon>()?;
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
}

pub(crate) struct Provider<T: Clone> {
    icon_size: u16,
    converter: fn(Icon) -> T,
    icons_cache: RefCell<BTreeMap<String, T>>,
}

impl<T: Clone> Provider<T> {
    pub fn new(icon_size: u16, converter: fn(Icon) -> T) -> Option<Self> {
        Some(Self {
            icon_size,
            converter,
            icons_cache: RefCell::new(BTreeMap::new()),
        })
    }

    pub fn get_file_icon(&self, path: impl AsRef<Path>) -> Option<T> {
        let path = path.as_ref();

        if path.is_dir()
            || path.is_symlink()
            || path
                .metadata()
                .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
        {
            return get_file_icon(path, self.icon_size).map(self.converter);
        }

        match path.extension().and_then(OsStr::to_str) {
            Some(".desktop") => get_file_icon(path, self.icon_size).map(self.converter),
            Some(extension) => match self.icons_cache.borrow_mut().entry(extension.to_owned()) {
                std::collections::btree_map::Entry::Vacant(vacant_entry) => Some(
                    vacant_entry
                        .insert(get_file_icon(path, self.icon_size).map(self.converter)?)
                        .clone(),
                ),
                std::collections::btree_map::Entry::Occupied(occupied_entry) => {
                    Some(occupied_entry.get().clone())
                }
            },
            None => get_file_icon(path, self.icon_size).map(self.converter),
        }
    }
}
