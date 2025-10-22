use crate::*;
use windows::Win32::Foundation::{LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
use windows::Win32::System::SystemServices::*;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(u32)]
enum MouseStateVirtualKey {
    Ctrl = 0x1,
    Shift = 0x2,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MouseStateVirtualKeys(u32);

impl MouseStateVirtualKeys {
    #[inline]
    pub fn contains(&self, key: VirtualKey) -> bool {
        if key == VirtualKey::Ctrl {
            self.0 & MouseStateVirtualKey::Ctrl as u32 != 0
        } else if key == VirtualKey::Shift {
            self.0 & MouseStateVirtualKey::Shift as u32 != 0
        } else {
            false
        }
    }

    #[inline]
    pub fn to_vec(&self) -> Vec<VirtualKey> {
        let mut v = vec![];
        if self.0 & MouseStateVirtualKey::Ctrl as u32 != 0 {
            v.push(VirtualKey::Ctrl);
        }
        if self.0 & MouseStateVirtualKey::Shift as u32 != 0 {
            v.push(VirtualKey::Shift);
        }
        v
    }
}

impl From<WPARAM> for MouseStateVirtualKeys {
    #[inline]
    fn from(value: WPARAM) -> Self {
        let value = loword(value.0 as i32) as u32;
        let mut ret = 0;
        if value & MK_CONTROL.0 != 0 {
            ret |= MouseStateVirtualKey::Ctrl as u32;
        }
        if value & MK_SHIFT.0 != 0 {
            ret |= MouseStateVirtualKey::Shift as u32;
        }
        Self(ret)
    }
}

impl std::fmt::Debug for MouseStateVirtualKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.to_vec();
        write!(f, "{v:?}")
    }
}

impl std::fmt::Display for MouseStateVirtualKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.to_vec();
        write!(f, "{v:?}")
    }
}

#[derive(Clone, Debug)]
pub struct MouseState {
    pub position: PhysicalPosition<i32>,
    pub buttons: MouseButtons,
    pub keys: MouseStateVirtualKeys,
}

impl MouseState {
    pub(crate) fn from_params(wparam: WPARAM, lparam: LPARAM) -> Self {
        Self {
            position: lparam_to_point(lparam),
            buttons: wparam.into(),
            keys: wparam.into(),
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
