use windows::Win32::UI::WindowsAndMessaging::WM_APP;

pub(crate) const WM_APP_POST_TASK: u32 = WM_APP;
pub(crate) const WM_APP_NOTIFY_ICON: u32 = WM_APP + 1;
pub(crate) const OFFSETED_WM_APP: u32 = WM_APP + 2;
