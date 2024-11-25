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

pub struct FileIconProvider<T: Clone> {
    cache: RefCell<BTreeMap<OsString, T>>,
    convert: fn(Icon) -> T,
}

impl<T: Clone> FileIconProvider<T> {
    pub fn new(convert: fn(Icon) -> T) -> Self {
        Self {
            cache: RefCell::new(BTreeMap::new()),
            convert,
        }
    }

    pub fn icon(&self, path: &Path) -> Option<T> {
        let get_icon = |path| get_file_icon(path).map(self.convert);

        match path.extension() {
            Some(extension) => match self.cache.borrow_mut().entry(extension.to_owned()) {
                Vacant(vacant_entry) => Some(vacant_entry.insert(get_icon(path)?).clone()),
                Occupied(occupied_entry) => Some(occupied_entry.get().clone()),
            },
            None => get_icon(path),
        }
    }
}
