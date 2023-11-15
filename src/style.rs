use windows::Win32::UI::WindowsAndMessaging::*;

/// Window styles are implement this trait.
pub trait Style {
    fn style(&self) -> WINDOW_STYLE;
    fn ex_style(&self) -> WINDOW_EX_STYLE;
}

fn set_style<T>(value: &mut T, style: T, flag: bool)
where
    T: std::ops::BitOrAssign + std::ops::BitAndAssign + std::ops::Not<Output = T>,
{
    if flag {
        *value |= style;
    } else {
        *value &= !style;
    }
}

/// Represents a borderless window style.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct BorderlessStyle {
    ex_style: WINDOW_EX_STYLE,
}

impl BorderlessStyle {
    #[inline]
    pub fn no_redirection_bitmap(mut self, flag: bool) -> Self {
        set_style(&mut self.ex_style, WS_EX_NOREDIRECTIONBITMAP, flag);
        self
    }
}

impl Style for BorderlessStyle {
    #[inline]
    fn style(&self) -> WINDOW_STYLE {
        WS_POPUP
    }

    #[inline]
    fn ex_style(&self) -> WINDOW_EX_STYLE {
        self.ex_style
    }
}

/// Represents a window style.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WindowStyle {
    style: WINDOW_STYLE,
    ex_style: WINDOW_EX_STYLE,
}

impl WindowStyle {
    #[inline]
    pub fn dialog() -> Self {
        Self {
            style: WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            ex_style: Default::default(),
        }
    }

    #[inline]
    pub fn borderless() -> BorderlessStyle {
        BorderlessStyle::default()
    }

    #[inline]
    pub fn resizable(mut self, flag: bool) -> Self {
        set_style(&mut self.style, WS_THICKFRAME, flag);
        self
    }

    #[inline]
    pub fn has_minimize_box(mut self, flag: bool) -> Self {
        set_style(&mut self.style, WS_MINIMIZEBOX, flag);
        self
    }

    #[inline]
    pub fn has_maximize_box(mut self, flag: bool) -> Self {
        set_style(&mut self.style, WS_MAXIMIZEBOX, flag);
        self
    }

    #[inline]
    pub fn no_redirection_bitmap(mut self, flag: bool) -> Self {
        set_style(&mut self.ex_style, WS_EX_NOREDIRECTIONBITMAP, flag);
        self
    }
}

impl Default for WindowStyle {
    #[inline]
    fn default() -> Self {
        Self {
            style: WS_OVERLAPPEDWINDOW,
            ex_style: Default::default(),
        }
    }
}

impl Style for WindowStyle {
    #[inline]
    fn style(&self) -> WINDOW_STYLE {
        self.style
    }

    #[inline]
    fn ex_style(&self) -> WINDOW_EX_STYLE {
        self.ex_style
    }
}
