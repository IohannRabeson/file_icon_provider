use std::path::Path;

use windows::Win32::Graphics::
            Gdi::{CreateCompatibleDC, DeleteDC, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC};

use crate::Icon;

#[allow(non_upper_case_globals)]
pub(crate) fn get_file_icon(path: impl AsRef<Path>, size: u16) -> Option<Icon> {
    use scopeguard::defer;
    use windows::{
        Win32::{
            Foundation::SIZE,
            Graphics::
                Gdi::DeleteObject
            ,
            UI::Shell::{
                IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY,
                SIIGBF_SCALEUP,
            },
        },
        core::HSTRING,
    };

    com::initialize()?;

    defer!(com::unitialize());

    let path = HSTRING::from(path.as_ref());
    let image_factory: IShellItemImageFactory =
        unsafe { SHCreateItemFromParsingName(&path, None) }.ok()?;
    let bitmap_size = SIZE {
        cx: size as i32,
        cy: size as i32,
    };
    let hbitmap = unsafe {
        image_factory
            .GetImage(bitmap_size, SIIGBF_ICONONLY | SIIGBF_SCALEUP)
            .ok()?
    };
    defer!(unsafe {
        let _ = DeleteObject(hbitmap);
    });

    let pixels = unsafe {
        let mut bmp: BITMAP = std::mem::zeroed();

        if GetObjectW(hbitmap, std::mem::size_of::<BITMAP>() as i32, Some(&mut bmp as *mut BITMAP as _)) == 0 {
            return None;
        }

        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
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
            return None;
        }

        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        pixels
    };

    Some(Icon {
        width: size as u32,
        height: size as u32,
        pixels,
    })
}

pub(crate) struct Provider<T: Clone> {
    icon_size: u16,
    converter: fn(Icon) -> T,
}

mod com {
    use std::cell::Cell;

    use windows::Win32::{
        System::Com::{CoInitialize, CoUninitialize},
    };

    std::thread_local! {
        static CO_INIT_COUNT: Cell<u32> = const { Cell::new(0) };
    }

    pub(crate) fn initialize() -> Option<()> {
        let count = CO_INIT_COUNT.get();
        if count == 0 {
            unsafe { CoInitialize(None) }.ok().ok()?;
        }

        CO_INIT_COUNT.set(count + 1);

        Some(())
    }

    pub(crate) fn unitialize() {
        let count = CO_INIT_COUNT.get();

        if count == 1 {
            unsafe { CoUninitialize() };
        }

        CO_INIT_COUNT.set(count - 1);
    }
}

impl<T: Clone> Provider<T> {
    pub fn new(icon_size: u16, converter: fn(Icon) -> T) -> Option<Self> {
        com::initialize();
        Some(Self {
            icon_size,
            converter,
        })
    }

    pub fn get_file_icon(&self, path: impl AsRef<Path>) -> Option<T> {
        get_file_icon(path, self.icon_size).map(self.converter)
    }
}

impl<T: Clone> Drop for Provider<T> {
    fn drop(&mut self) {
        com::unitialize();
    }
}
