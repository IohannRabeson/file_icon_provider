use objc2::{AnyThread, rc::Retained};
use objc2_app_kit::{
    NSBitmapImageRep, NSCompositingOperation, NSGraphicsContext, NSImage, NSWorkspace,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use objc2_uniform_type_identifiers::UTType;

use crate::Icon;
use std::{
    cell::RefCell,
    collections::{BTreeMap, btree_map},
    path::Path,
};

use log::error;

pub(crate) fn get_file_icon(path: impl AsRef<Path>, size: u16) -> Option<Icon> {
    let file_path = path_to_nsstring(path)?;
    let shared_workspace = NSWorkspace::sharedWorkspace();
    let image = shared_workspace.iconForFile(&file_path);
    let bitmap_representation = create_bitmap_representation(size)?;
    let context = create_context(&bitmap_representation)?;

    Some(Icon {
        width: size as u32,
        height: size as u32,
        pixels: get_pixels(&image, &context, &bitmap_representation, size as u32)?,
    })
}

pub struct Provider<T: Clone> {
    shared_workspace: Retained<NSWorkspace>,
    bitmap_representation: Retained<NSBitmapImageRep>,
    context: Option<Retained<NSGraphicsContext>>,
    icon_size: u32,
    cache: RefCell<BTreeMap<String, T>>,
    converter: fn(Icon) -> T,
}

impl<T> Provider<T>
where
    T: Clone,
{
    pub fn new(icon_size: u16, converter: fn(Icon) -> T) -> Option<Self> {
        let mut provider = Self {
            shared_workspace: NSWorkspace::sharedWorkspace(),
            bitmap_representation: create_bitmap_representation(icon_size)?,
            context: None,
            icon_size: icon_size as u32,
            cache: RefCell::new(BTreeMap::new()),
            converter,
        };

        provider.context = create_context(&provider.bitmap_representation);

        Some(provider)
    }

    pub fn get_file_icon(&self, path: impl AsRef<Path>) -> Option<T> {
        match Self::get_uttype_identifier(&path) {
            Some(identifier) => match self.cache.borrow_mut().entry(identifier) {
                btree_map::Entry::Vacant(vacant_entry) => {
                    let icon = self.get_icon(path)?;

                    Some(vacant_entry.insert(icon).clone())
                }
                btree_map::Entry::Occupied(occupied_entry) => Some(occupied_entry.get().clone()),
            },
            None => self.get_icon(path),
        }
    }

    fn get_uttype_identifier(path: impl AsRef<Path>) -> Option<String> {
        if path.as_ref().is_dir() {
            return None;
        }

        let extension = NSString::from_str(path.as_ref().extension()?.to_str()?);
        let ut_type = UTType::typeWithFilenameExtension(&extension)?;

        Some(ut_type.identifier().to_string())
    }

    pub fn get_icon(&self, path: impl AsRef<Path>) -> Option<T> {
        let path = path.as_ref();
        let context = self.context.as_ref().unwrap();
        let file_path = path_to_nsstring(path)?;
        let image = self.shared_workspace.iconForFile(&file_path);

        Some((self.converter)(Icon {
            width: self.icon_size,
            height: self.icon_size,
            pixels: get_pixels(&image, context, &self.bitmap_representation, self.icon_size)?,
        }))
    }
}

fn create_bitmap_representation(icon_size: u16) -> Option<Retained<NSBitmapImageRep>> {
    Some(
        match unsafe {
            let color_space_name = NSString::from_str("NSDeviceRGBColorSpace");

            NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
        NSBitmapImageRep::alloc(),
        std::ptr::null_mut(),
        icon_size as isize,
        icon_size as isize,
        8,
        4,
        true,
        false,
        &color_space_name,
        icon_size as isize * 4,
        32,
    )
        } {
            Some(bitmap_representation) => bitmap_representation,
            None => {
                error!("Failed to create NSBitmapImageRep");
                return None;
            }
        },
    )
}

fn create_context(
    bitmap_representation: &Retained<NSBitmapImageRep>,
) -> Option<Retained<NSGraphicsContext>> {
    Some(
        match NSGraphicsContext::graphicsContextWithBitmapImageRep(bitmap_representation)
        {
            Some(context) => context,
            None => {
                error!("Failed to create graphics context");
                return None;
            }
        },
    )
}

fn get_pixels(
    image: &NSImage,
    context: &NSGraphicsContext,
    bitmap_representation: &NSBitmapImageRep,
    icon_size: u32,
) -> Option<Vec<u8>> {
    let image_size = image.size();

    if image_size.width < 1.0 || image_size.height < 1.0 {
        error!("Invalid image size");
        return None;
    }

    let desired_size = NSSize {
        width: icon_size as f64,
        height: icon_size as f64,
    };

    Some(unsafe {
        context.saveGraphicsState();
        NSGraphicsContext::setCurrentContext(Some(context));
        image.setSize(desired_size);
        image.drawAtPoint_fromRect_operation_fraction(
            NSPoint::ZERO,
            NSRect::new(NSPoint::ZERO, desired_size),
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
    })
}

fn path_to_nsstring(path: impl AsRef<Path>) -> Option<Retained<NSString>> {
    let path = match path.as_ref().canonicalize() {
        Ok(path) => path,
        Err(error) => {
            error!(
                "Failed to canonicalize path '{}': {}",
                path.as_ref().display(),
                error
            );
            return None;
        }
    };

    match path.to_str() {
        Some(path) => Some(NSString::from_str(path)),
        None => {
            error!("Path '{}' is not valid unicode", path.display());
            None
        }
    }
}
