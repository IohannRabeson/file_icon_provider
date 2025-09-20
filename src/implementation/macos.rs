use objc2::{AnyThread, rc::Retained};
use objc2_app_kit::{NSBitmapImageRep, NSCompositingOperation, NSGraphicsContext, NSWorkspace};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use objc2_uniform_type_identifiers::UTType;

use crate::Icon;
use std::{
    cell::RefCell,
    collections::{BTreeMap, btree_map},
    path::Path,
};

pub(crate) fn get_file_icon(path: impl AsRef<Path>, size: u16) -> Option<Icon> {
    let path = path.as_ref().canonicalize().ok()?;
    let file_path = NSString::from_str(path.to_str()?);
    let color_space_name = NSString::from_str("NSDeviceRGBColorSpace");
    let shared_workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let image = unsafe { shared_workspace.iconForFile(&file_path) };
    let image_size = unsafe { image.size() };
    let desired_size = NSSize {
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
        let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap_representation)?;

        context.saveGraphicsState();

        NSGraphicsContext::setCurrentContext(Some(&context));

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
    };

    Some(Icon {
        width: size as u32,
        height: size as u32,
        pixels,
    })
}

pub struct Provider<T: Clone> {
    shared_workspace: Retained<NSWorkspace>,
    bitmap_representation: Retained<NSBitmapImageRep>,
    context: Option<Retained<NSGraphicsContext>>,
    desired_size: NSSize,
    icon_size: u32,
    cache: RefCell<BTreeMap<String, T>>,
    converter: fn(Icon) -> T,
}

impl<T> Provider<T>
where
    T: Clone,
{
    pub fn new(icon_size: u16, converter: fn(Icon) -> T) -> Option<Self> {
        let color_space_name = NSString::from_str("NSDeviceRGBColorSpace");
        let mut provider = Self {
            shared_workspace: unsafe { NSWorkspace::sharedWorkspace() },
            bitmap_representation: unsafe {
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
            )?
            },
            context: None,
            desired_size: NSSize {
                width: icon_size as f64,
                height: icon_size as f64,
            },
            icon_size: icon_size as u32,
            cache: RefCell::new(BTreeMap::new()),
            converter,
        };

        provider.context = Some(unsafe {
            NSGraphicsContext::graphicsContextWithBitmapImageRep(&provider.bitmap_representation)?
        });

        Some(provider)
    }

    pub fn get_file_icon(&self, path: impl AsRef<Path>) -> Option<T> {
        match Self::get_uttype_identifier(&path) {
            Some(identifier) => match self.cache.borrow_mut().entry(identifier) {
                btree_map::Entry::Vacant(vacant_entry) => {
                    let icon = self.get_icon(path)?;

                    return Some(vacant_entry.insert(icon).clone());
                }
                btree_map::Entry::Occupied(occupied_entry) => {
                    return Some(occupied_entry.get().clone());
                }
            },
            None => self.get_icon(path),
        }
    }

    fn get_uttype_identifier(path: impl AsRef<Path>) -> Option<String> {
        if path.as_ref().is_dir() {
            return None;
        }

        let extension = NSString::from_str(path.as_ref().extension()?.to_str()?);
        let ut_type = unsafe { UTType::typeWithFilenameExtension(&*extension) }?;

        Some(unsafe { ut_type.identifier().to_string() })
    }

    pub fn get_icon(&self, path: impl AsRef<Path>) -> Option<T> {
        let path = path.as_ref();

        let pixels = unsafe {
            let context = self.context.as_ref().unwrap();
            let file_path = NSString::from_str(path.to_str()?);
            let image = self.shared_workspace.iconForFile(&file_path);

            context.saveGraphicsState();
            NSGraphicsContext::setCurrentContext(Some(context));
            image.setSize(self.desired_size);
            image.drawAtPoint_fromRect_operation_fraction(
                NSPoint::ZERO,
                NSRect::new(NSPoint::ZERO, self.desired_size),
                NSCompositingOperation::Copy,
                1.0,
            );
            context.flushGraphics();
            context.restoreGraphicsState();

            std::slice::from_raw_parts(
                self.bitmap_representation.bitmapData(),
                self.bitmap_representation.bytesPerPlane() as usize,
            )
            .to_vec()
        };

        Some((self.converter)(Icon {
            width: self.icon_size,
            height: self.icon_size,
            pixels,
        }))
    }
}
