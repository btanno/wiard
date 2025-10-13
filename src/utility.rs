use crate::*;
use std::sync::Arc;
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::{MONITOR_DEFAULTTOPRIMARY, MonitorFromPoint},
    System::LibraryLoader::*,
    UI::Controls::{CloseThemeData, HTHEME, OpenThemeData},
    UI::HiDpi::*,
    UI::WindowsAndMessaging::*,
};
use windows::core::{HSTRING, PCSTR};

pub fn adjust_window_rect_ex_for_dpi(
    size: impl ToPhysical<u32, Output<u32> = PhysicalSize<u32>>,
    style: WINDOW_STYLE,
    has_menu: bool,
    ex_style: WINDOW_EX_STYLE,
    dpi: u32,
) -> RECT {
    let size = size.to_physical(dpi);
    let mut rc = RECT {
        right: size.width as i32,
        bottom: size.height as i32,
        ..Default::default()
    };
    unsafe {
        AdjustWindowRectExForDpi(&mut rc, style, has_menu, ex_style, dpi).ok();
    }
    rc
}

pub fn get_client_rect(hwnd: HWND) -> RECT {
    let mut rc = RECT::default();
    unsafe {
        GetClientRect(hwnd, &mut rc).ok();
    }
    rc
}

pub fn get_window_rect(hwnd: HWND) -> RECT {
    let mut rc = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut rc).ok();
    }
    rc
}

pub fn get_dpi_from_point(pt: ScreenPosition<i32>) -> u32 {
    let mut x = 0;
    let mut y = 0;
    unsafe {
        GetDpiForMonitor(
            MonitorFromPoint(POINT { x: pt.x, y: pt.y }, MONITOR_DEFAULTTOPRIMARY),
            MDT_DEFAULT,
            &mut x,
            &mut y,
        )
        .ok();
    }
    x
}

pub fn loword(x: i32) -> i16 {
    (x & 0xffff) as _
}

pub fn hiword(x: i32) -> i16 {
    ((x >> 16) & 0xffff) as _
}

pub fn get_x_lparam(lp: LPARAM) -> i16 {
    (lp.0 & 0xffff) as _
}

pub fn get_y_lparam(lp: LPARAM) -> i16 {
    ((lp.0 >> 16) & 0xffff) as _
}

pub fn get_xbutton_wparam(wp: WPARAM) -> u16 {
    ((wp.0 >> 16) & 0xffff) as _
}

pub fn lparam_to_point<C>(lparam: LPARAM) -> Position<i32, C> {
    Position::new(get_x_lparam(lparam) as _, get_y_lparam(lparam) as _)
}

pub fn lparam_to_size(lparam: LPARAM) -> PhysicalSize<u32> {
    Size::new(get_x_lparam(lparam) as _, get_y_lparam(lparam) as _)
}

#[derive(Debug)]
struct RawTheme(HTHEME);

impl Drop for RawTheme {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseThemeData(self.0);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Theme {
    handle: Arc<RawTheme>,
}

impl Theme {
    #[inline]
    pub fn new(hwnd: HWND, classes: &[&str]) -> Self {
        let classes = HSTRING::from(classes.join(";"));
        let handle = unsafe { OpenThemeData(Some(hwnd), &classes) };
        Self {
            handle: Arc::new(RawTheme(handle)),
        }
    }

    #[inline]
    pub fn handle(&self) -> HTHEME {
        self.handle.0
    }
}

#[derive(Clone)]
pub struct Symbol<F> {
    ptr: FARPROC,
    _f: std::marker::PhantomData<F>,
}

impl<F> Symbol<F> {
    fn new(ptr: FARPROC) -> Self {
        Self {
            ptr,
            _f: std::marker::PhantomData,
        }
    }
}

impl<F> std::ops::Deref for Symbol<F> {
    type Target = F;

    fn deref(&self) -> &F {
        unsafe { &*(&self.ptr as *const FARPROC as *const F) }
    }
}

pub struct Library(HMODULE);

impl Library {
    pub fn new(lib: &str) -> windows::core::Result<Self> {
        unsafe {
            let handle = LoadLibraryExW(&HSTRING::from(lib), None, LOAD_LIBRARY_SEARCH_SYSTEM32)?;
            Ok(Self(handle))
        }
    }

    pub fn get_proc_address<F>(&self, name: PCSTR) -> Symbol<F> {
        unsafe { Symbol::new(GetProcAddress(self.0, name)) }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            let _ = FreeLibrary(self.0);
        }
    }
}

unsafe impl Send for Library {}
unsafe impl Sync for Library {}
