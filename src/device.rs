use crate::*;
use windows::Win32::Foundation::{LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

#[derive(Clone, Debug)]
pub struct MouseState {
    pub position: PhysicalPosition<i32>,
    pub buttons: MouseButtons,
}

impl MouseState {
    pub(crate) fn from_params(wparam: WPARAM, lparam: LPARAM) -> Self {
        Self {
            position: lparam_to_point(lparam),
            buttons: wparam.into(),
        }
    }
}

/// This value is a multiple of wheel value.
pub const WHEEL_DELTA: i32 = windows::Win32::UI::WindowsAndMessaging::WHEEL_DELTA as i32;

#[inline]
pub fn get_cursor_pos() -> ScreenPosition<i32> {
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        ScreenPosition::new(pt.x, pt.y)
    }
}

#[inline]
pub fn client_to_screen(window: &impl IsWindow, src: PhysicalPosition<i32>) -> ScreenPosition<i32> {
    unsafe {
        let mut pt = POINT { x: src.x, y: src.y };
        let _ = ClientToScreen(window.window_handle().as_hwnd(), &mut pt);
        ScreenPosition::new(pt.x, pt.y)
    }
}

#[inline]
pub fn screen_to_client(window: &impl IsWindow, src: ScreenPosition<i32>) -> PhysicalPosition<i32> {
    unsafe {
        let mut pt = POINT { x: src.x, y: src.y };
        let _ = ScreenToClient(window.window_handle().as_hwnd(), &mut pt);
        PhysicalPosition::new(pt.x, pt.y)
    }
}
