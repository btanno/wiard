use super::*;
use std::sync::LazyLock;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_DWORD, REG_VALUE_TYPE, RRF_RT_REG_DWORD, RegGetValueW,
};
use windows::core::PCSTR;

/// Represents a color mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum ColorMode {
    System,
    Light,
    Dark,
}

/// Represents a color mode state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub enum ColorModeState {
    Light = ColorMode::Light as i32,
    Dark = ColorMode::Dark as i32,
}

/// Check the dark mode in Windows.
pub fn is_system_dark_mode() -> bool {
    let key =
        windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value = windows::core::w!("AppsUseLightTheme");
    unsafe {
        let mut ty = REG_VALUE_TYPE::default();
        let mut data = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        let ret = RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            value,
            RRF_RT_REG_DWORD,
            Some(&mut ty),
            Some(&mut data as *mut u32 as *mut std::ffi::c_void),
            Some(&mut size),
        )
        .ok();
        if let Err(e) = ret {
            error!("{e}");
            return false;
        }
        ty == REG_DWORD && data == 0
    }
}

static UXTHEME: LazyLock<Library> = LazyLock::new(|| Library::new("uxtheme.dll").unwrap());

#[allow(dead_code)]
pub(crate) const APPMODE_DEFAULT: i32 = 0;
pub(crate) const APPMODE_ALLOWDARK: i32 = 1;
pub(crate) const APPMODE_FORCEDARK: i32 = 2;
pub(crate) const APPMODE_FORCELIGHT: i32 = 3;

#[inline]
pub(crate) fn refresh_immersive_color_policy_state() {
    unsafe {
        static FUNC: LazyLock<Symbol<unsafe extern "system" fn()>> =
            LazyLock::new(|| UXTHEME.get_proc_address(PCSTR(104 as *const u8)));
        FUNC()
    }
}

#[inline]
pub(crate) fn set_preferred_app_mode(app_mode: i32) -> i32 {
    unsafe {
        static FUNC: LazyLock<Symbol<unsafe extern "system" fn(i32) -> i32>> =
            LazyLock::new(|| UXTHEME.get_proc_address(PCSTR(135 as *const u8)));
        FUNC(app_mode)
    }
}
