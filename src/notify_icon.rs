use super::*;
use std::sync::atomic::{self, AtomicU32};
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
struct Id(u32);

impl Id {
    fn new() -> Self {
        static GEN_ID: AtomicU32 = AtomicU32::new(0);
        Self(GEN_ID.fetch_add(1, atomic::Ordering::SeqCst))
    }
}

pub struct Builder<'a> {
    window: WindowHandle,
    icon: Option<&'a Icon>,
    tip: Option<String>,
}

impl Builder<'_> {
    fn new(window: &impl IsWindow) -> Self {
        Self {
            window: window.window_handle(),
            icon: None,
            tip: None,
        }
    }

    #[inline]
    pub fn icon(self, icon: &Icon) -> Builder<'_> {
        Builder {
            window: self.window,
            icon: Some(icon),
            tip: self.tip,
        }
    }

    #[inline]
    pub fn tip(mut self, s: impl Into<String>) -> Self {
        self.tip = Some(s.into());
        self
    }

    #[inline]
    pub fn build(self) -> Result<NotifyIcon> {
        let id = Id::new();
        unsafe {
            let mut data = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.window.as_hwnd(),
                uID: id.0,
                uFlags: NIF_MESSAGE,
                uCallbackMessage: WM_APP_NOTIFY_ICON,
                Anonymous: NOTIFYICONDATAW_0 {
                    uVersion: NOTIFYICON_VERSION_4,
                },
                ..Default::default()
            };
            if let Some(icon) = self.icon {
                let hinstance: Option<HINSTANCE> = Some(GetModuleHandleW(None).unwrap().into());
                data.uFlags |= NIF_ICON;
                data.hIcon = icon.load(hinstance)?;
            }
            if let Some(tip) = self.tip {
                data.uFlags |= NIF_TIP | NIF_SHOWTIP;
                let tip = tip
                    .encode_utf16()
                    .take(data.szTip.len() - 1)
                    .chain(std::iter::once(0));
                for (i, c) in tip.enumerate() {
                    data.szTip[i] = c;
                }
            }
            Shell_NotifyIconW(NIM_ADD, &data).ok()?;
            Shell_NotifyIconW(NIM_SETVERSION, &data).ok()?;
        }
        Ok(NotifyIcon { id })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct NotifyIcon {
    id: Id,
}

impl NotifyIcon {
    #[inline]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(window: &impl IsWindow) -> Builder<'_> {
        Builder::new(window)
    }

    pub(crate) fn from_id(id: u32) -> Self {
        Self { id: Id(id) }
    }

    #[inline]
    pub fn delete(self) -> bool {
        unsafe {
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                uID: self.id.0,
                uCallbackMessage: WM_APP_NOTIFY_ICON,
                ..Default::default()
            };
            Shell_NotifyIconW(NIM_DELETE, &nid).as_bool()
        }
    }
}

pub mod event {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct MouseInput {
        pub button: MouseButton,
        pub button_state: ButtonState,
        pub position: ScreenPosition<i32>,
    }
}

#[derive(Clone, Debug)]
pub enum NotifyIconEvent {
    MouseInput(event::MouseInput),
    CursorMoved(ScreenPosition<i32>),
    ContextMenu(ScreenPosition<i32>),
    PopupOpen(ScreenPosition<i32>),
    PopupClose,
    Select(ScreenPosition<i32>),
    KeySelect(ScreenPosition<i32>),
    Other(super::event::Other),
}
