use crate::*;
use windows::Win32::Foundation::{LPARAM, WPARAM, POINT};
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