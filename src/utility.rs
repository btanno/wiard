use crate::*;
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTOPRIMARY},
    UI::HiDpi::*,
    UI::WindowsAndMessaging::*,
};

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
