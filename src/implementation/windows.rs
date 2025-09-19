use std::{
    cell::RefCell,
    collections::BTreeMap,
    ffi::OsStr,
    path::Path,
    sync::{
        LazyLock,
        mpsc::{Sender, channel},
    },
};

use scopeguard::defer;
use windows::{
    Win32::{
        Foundation::SIZE,
        Graphics::Gdi::{
            BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS,
            DeleteDC, DeleteObject, GetDIBits, GetObjectW, HDC,
        },
        System::Com::{CoInitialize, CoUninitialize},
        UI::Shell::{
            IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY, SIIGBF_SCALEUP,
        },
    },
    core::HSTRING,
};

use crate::Icon;

enum ImageFactoryRequest {
    RequestImage {
        path: HSTRING,
        size: u16,
        reply: Sender<ImageFactoryReply>,
    },
}

enum ImageFactoryReply {
    Success(Icon),
    Failure,
}

static IMAGE_FACTORY_REQUEST_SENDER: LazyLock<Sender<ImageFactoryRequest>> =
    LazyLock::new(start_image_factory_thread);

fn start_image_factory_thread() -> Sender<ImageFactoryRequest> {
    let (sender, receiver) = channel();

    std::thread::spawn(move || {
        while let Ok(request) = receiver.recv() {
            match request {
                ImageFactoryRequest::RequestImage { path, size, reply } => {
                    if unsafe { CoInitialize(None) }.is_err() {
                        let _ = reply.send(ImageFactoryReply::Failure);
                        continue;
                    }

                    defer!(unsafe { CoUninitialize() });

                    let factory: Result<IShellItemImageFactory, _> =
                        unsafe { SHCreateItemFromParsingName(&path, None) };
                    match factory {
                        Ok(factory) => {
                            match unsafe {
                                factory.GetImage(
                                    SIZE {
                                        cx: size as i32,
                                        cy: size as i32,
                                    },
                                    SIIGBF_ICONONLY | SIIGBF_SCALEUP,
                                )
                            } {
                                Ok(hbitmap) => {
                                    let pixels = unsafe {
                                        defer!({
                                            let _ = DeleteObject(hbitmap.into());
                                        });

                                        let mut bmp: BITMAP = std::mem::zeroed();

                                        if GetObjectW(
                                            hbitmap.into(),
                                            std::mem::size_of::<BITMAP>() as i32,
                                            Some(&mut bmp as *mut BITMAP as _),
                                        ) == 0
                                        {
                                            continue;
                                        }

                                        let mut bi: BITMAPINFO = std::mem::zeroed();
                                        bi.bmiHeader.biSize =
                                            std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                                        bi.bmiHeader.biWidth = bmp.bmWidth;
                                        bi.bmiHeader.biHeight = -bmp.bmHeight;
                                        bi.bmiHeader.biPlanes = 1;
                                        bi.bmiHeader.biBitCount = 32;
                                        bi.bmiHeader.biCompression = BI_RGB.0;

                                        let stride = (bmp.bmWidth * 4) as usize;
                                        let mut pixels = vec![0u8; stride * bmp.bmHeight as usize];
                                        let hdc: HDC = CreateCompatibleDC(None);
                                        let res = GetDIBits(
                                            hdc,
                                            hbitmap,
                                            0,
                                            bmp.bmHeight as u32,
                                            Some(pixels.as_mut_ptr() as _),
                                            &mut bi,
                                            DIB_RGB_COLORS,
                                        );

                                        let _ = DeleteDC(hdc);

                                        if res == 0 {
                                            continue;
                                        }

                                        for chunk in pixels.chunks_exact_mut(4) {
                                            chunk.swap(0, 2);
                                        }

                                        pixels
                                    };

                                    let _ = reply.send(ImageFactoryReply::Success(Icon {
                                        width: size as u32,
                                        height: size as u32,
                                        pixels,
                                    }));
                                }
                                Err(_) => {
                                    let _ = reply.send(ImageFactoryReply::Failure);
                                }
                            }
                        }
                        Err(_) => {
                            let _ = reply.send(ImageFactoryReply::Failure);
                        }
                    }
                }
            }
        }
    });

    sender
}

#[allow(non_upper_case_globals)]
pub(crate) fn get_file_icon(path: impl AsRef<Path>, size: u16) -> Option<Icon> {
    let path = HSTRING::from(path.as_ref());
    let (reply_tx, reply_rx) = channel();

    IMAGE_FACTORY_REQUEST_SENDER
        .send(ImageFactoryRequest::RequestImage {
            path,
            size,
            reply: reply_tx,
        })
        .unwrap();

    let icon = match reply_rx.recv() {
        Ok(ImageFactoryReply::Success(icon)) => icon,
        Ok(ImageFactoryReply::Failure) => return None,
        Err(_) => return None,
    };

    Some(icon)
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

        match path.extension().and_then(OsStr::to_str) {
            // On Windows .exe and .lnk can have any icon so they are never cached.
            Some(".exe") | Some(".lnk") => get_file_icon(path, self.icon_size).map(self.converter),
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
