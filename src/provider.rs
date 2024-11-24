use std::{collections::{btree_map::Entry::{Occupied, Vacant}, BTreeMap}, ffi::OsStr, path::Path};

use crate::{get_file_icon, Icon};

pub struct FileIconProvider<'a, T: Clone> {
    cache: BTreeMap<&'a OsStr, T>,
    convert: fn(Icon) -> T,
}

impl<'a, T: Clone> FileIconProvider<'a, T> {
    pub fn new(convert: fn(Icon) -> T) -> Self {
        Self {
            cache: BTreeMap::new(),
            convert,
        }
    }

    pub fn icon(&mut self, path: &'a Path) -> Option<T> {
        let get_icon = |path| get_file_icon(path).map(self.convert);

        match path.extension() {
            Some(extension) => {
                match self.cache.entry(extension) {
                    Vacant(vacant_entry) => {
                        Some(vacant_entry.insert(get_icon(path)?).clone())
                    },
                    Occupied(occupied_entry) => {
                        Some(occupied_entry.get().clone())
                    },
                }
            }
            None => get_icon(path),
        }        
    }
}