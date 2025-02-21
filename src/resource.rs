use crate::*;
use std::path::{Path, PathBuf};
use windows::Win32::{Foundation::HINSTANCE, UI::WindowsAndMessaging::*};
use windows::core::{HSTRING, PCWSTR};

/// Represents icons.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Icon {
    Resource(u16),
    File(PathBuf),
    Application,
    Error,
    Warning,
    Information,
    Question,
    WinLogo,
    Shield,
}

impl Icon {
    /// An icon image from an file.
    #[inline]
    pub fn from_path(path: impl AsRef<Path>) -> Icon {
        Icon::File(path.as_ref().into())
    }

    pub(crate) fn load(&self, hinst: Option<HINSTANCE>) -> Result<HICON> {
        unsafe {
            self.load_impl(
                hinst,
                GetSystemMetrics(SM_CXICON),
                GetSystemMetrics(SM_CYICON),
            )
        }
    }

    pub(crate) fn load_small(&self, hinst: Option<HINSTANCE>) -> Result<HICON> {
        unsafe {
            self.load_impl(
                hinst,
                GetSystemMetrics(SM_CXSMICON),
                GetSystemMetrics(SM_CYSMICON),
            )
        }
    }

    fn load_impl(&self, hinst: Option<HINSTANCE>, cx: i32, cy: i32) -> Result<HICON> {
        unsafe {
            match self {
                Icon::Resource(id) => {
                    let handle = LoadImageW(
                        hinst,
                        PCWSTR(*id as *const u16),
                        IMAGE_ICON,
                        cx,
                        cy,
                        LR_SHARED,
                    )?;
                    Ok(HICON(handle.0))
                }
                Icon::File(path) => {
                    let path = path.to_string_lossy();
                    let handle = LoadImageW(
                        None,
                        &HSTRING::from(path.as_ref()),
                        IMAGE_ICON,
                        cx,
                        cy,
                        LR_SHARED | LR_LOADFROMFILE,
                    )?;
                    Ok(HICON(handle.0))
                }
                Icon::Application => Ok(LoadIconW(None, IDI_APPLICATION)?),
                Icon::Error => Ok(LoadIconW(None, IDI_ERROR)?),
                Icon::Warning => Ok(LoadIconW(None, IDI_WARNING)?),
                Icon::Information => Ok(LoadIconW(None, IDI_INFORMATION)?),
                Icon::Question => Ok(LoadIconW(None, IDI_QUESTION)?),
                Icon::WinLogo => Ok(LoadIconW(None, IDI_WINLOGO)?),
                Icon::Shield => Ok(LoadIconW(None, IDI_SHIELD)?),
            }
        }
    }
}

/// Represents mouse cursors.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Cursor {
    AppStaring,
    Arrow,
    Cross,
    Hand,
    Help,
    IBeam,
    No,
    SizeAll,
    SizeNESW,
    SizeNS,
    SizeNWSE,
    SizeWE,
    UpArrow,
    Wait,
}

impl Cursor {
    fn system_defined_name(&self) -> PCWSTR {
        match self {
            Self::AppStaring => IDC_APPSTARTING,
            Self::Arrow => IDC_ARROW,
            Self::Cross => IDC_CROSS,
            Self::Hand => IDC_HAND,
            Self::Help => IDC_HELP,
            Self::IBeam => IDC_IBEAM,
            Self::No => IDC_NO,
            Self::SizeAll => IDC_SIZEALL,
            Self::SizeNESW => IDC_SIZENESW,
            Self::SizeNS => IDC_SIZENS,
            Self::SizeNWSE => IDC_SIZENWSE,
            Self::SizeWE => IDC_SIZEWE,
            Self::UpArrow => IDC_UPARROW,
            Self::Wait => IDC_WAIT,
        }
    }

    pub(crate) fn set(&self) {
        unsafe {
            SetCursor(Some(LoadCursorW(None, self.system_defined_name()).unwrap()));
        }
    }
}

impl Default for Cursor {
    #[inline]
    fn default() -> Self {
        Self::Arrow
    }
}
