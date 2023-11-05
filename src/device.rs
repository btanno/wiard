use crate::*;
use windows::Win32::{Foundation::{WPARAM, LPARAM}, UI::Input::KeyboardAndMouse::*};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum KeyState {
    Pressed,
    Released,
}

pub type ButtonState = KeyState;

impl std::fmt::Display for KeyState {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Ex(u32),
}

impl MouseButton {
    const EX_LEN: u32 = 29;

    fn as_u32(&self) -> u32 {
        match self {
            Self::Left => 0x01,
            Self::Right => 0x01 << 1,
            Self::Middle => 0x01 << 2,
            Self::Ex(x) => 0x01 << (x + 3),
        }
    }
}

impl std::fmt::Display for MouseButton {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseButtons(pub u32);

impl MouseButtons {
    #[inline]
    pub fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn contains(&self, btn: MouseButton) -> bool {
        let btn = btn.as_u32();
        self.0 & btn == btn
    }
}

impl std::ops::BitOr for MouseButton {
    type Output = MouseButtons;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        MouseButtons(self.as_u32() | rhs.as_u32())
    }
}

impl std::ops::BitOr<MouseButtons> for MouseButton {
    type Output = MouseButtons;

    #[inline]
    fn bitor(self, rhs: MouseButtons) -> Self::Output {
        MouseButtons(self.as_u32() | rhs.0)
    }
}

impl std::ops::BitOr<MouseButton> for MouseButtons {
    type Output = MouseButtons;

    #[inline]
    fn bitor(self, rhs: MouseButton) -> Self::Output {
        MouseButtons(self.0 | rhs.as_u32())
    }
}

impl std::fmt::Debug for MouseButtons {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MouseButtons({self})")
    }
}

impl std::fmt::Display for MouseButtons {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut v: Vec<String> = vec![];
        if self.contains(MouseButton::Left) {
            v.push("Left".into());
        }
        if self.contains(MouseButton::Right) {
            v.push("Right".into());
        }
        if self.contains(MouseButton::Middle) {
            v.push("Middle".into());
        }
        for i in 0..MouseButton::EX_LEN {
            if self.contains(MouseButton::Ex(i)) {
                v.push(format!("Ex({i})"));
            }
        }
        write!(f, "MouseButtons({})", v.join(" | "))
    }
}

impl Default for MouseButtons {
    #[inline]
    fn default() -> Self {
        MouseButtons::new()
    }
}

impl<const N: usize> From<[MouseButton; N]> for MouseButtons {
    #[inline]
    fn from(value: [MouseButton; N]) -> Self {
        value.iter().fold(MouseButtons::new(), |r, b| r | *b)
    }
}

impl<const N: usize> From<&[MouseButton; N]> for MouseButtons {
    #[inline]
    fn from(value: &[MouseButton; N]) -> Self {
        value.iter().fold(MouseButtons::new(), |r, b| r | *b)
    }
}

impl From<&[MouseButton]> for MouseButtons {
    #[inline]
    fn from(value: &[MouseButton]) -> Self {
        value.iter().fold(MouseButtons::new(), |r, b| r | *b)
    }
}

impl From<Vec<MouseButton>> for MouseButtons {
    #[inline]
    fn from(value: Vec<MouseButton>) -> Self {
        value.iter().fold(MouseButtons::new(), |r, b| r | *b)
    }
}

impl From<&Vec<MouseButton>> for MouseButtons {
    #[inline]
    fn from(value: &Vec<MouseButton>) -> Self {
        value.iter().fold(MouseButtons::new(), |r, b| r | *b)
    }
}

impl From<WPARAM> for MouseButtons {
    #[inline]
    fn from(value: WPARAM) -> Self {
        Self((value.0 & 0xffff) as u32)
    }
}

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MouseWheelAxis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VirtualKey {
    Esc,
    Tab,
    CapsLock,
    Shift,
    Ctrl,
    Alt,
    BackSpace,
    Enter,
    Space,
    PrintScreen,
    ScrollLock,
    Pause,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    NumLock,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Num0,
    NumAdd,
    NumSub,
    NumMul,
    NumDiv,
    NumDecimal,
    Other(u32),
}

impl std::fmt::Display for VirtualKey {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ScanCode(pub u32);

#[derive(Clone, Copy, Debug)]
pub struct KeyCode {
    pub vkey: VirtualKey,
    pub scan_code: ScanCode,
}

impl KeyCode {
    #[inline]
    pub fn new(vkey: VirtualKey, scan_code: ScanCode) -> Self {
        Self { vkey, scan_code }
    }
}

impl PartialEq for KeyCode {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.vkey == other.vkey
    }
}

impl Eq for KeyCode {}

impl From<VIRTUAL_KEY> for VirtualKey {
    fn from(value: VIRTUAL_KEY) -> Self {
        match value {
            VK_1 => VirtualKey::Key1,
            VK_2 => VirtualKey::Key2,
            VK_3 => VirtualKey::Key3,
            VK_4 => VirtualKey::Key4,
            VK_5 => VirtualKey::Key5,
            VK_6 => VirtualKey::Key6,
            VK_7 => VirtualKey::Key7,
            VK_8 => VirtualKey::Key8,
            VK_9 => VirtualKey::Key9,
            VK_0 => VirtualKey::Key0,
            VK_A => VirtualKey::A,
            VK_B => VirtualKey::B,
            VK_C => VirtualKey::C,
            VK_D => VirtualKey::D,
            VK_E => VirtualKey::E,
            VK_F => VirtualKey::F,
            VK_G => VirtualKey::G,
            VK_H => VirtualKey::H,
            VK_I => VirtualKey::I,
            VK_J => VirtualKey::J,
            VK_K => VirtualKey::K,
            VK_L => VirtualKey::L,
            VK_M => VirtualKey::M,
            VK_N => VirtualKey::N,
            VK_O => VirtualKey::O,
            VK_P => VirtualKey::P,
            VK_Q => VirtualKey::Q,
            VK_R => VirtualKey::R,
            VK_S => VirtualKey::S,
            VK_T => VirtualKey::T,
            VK_U => VirtualKey::U,
            VK_V => VirtualKey::V,
            VK_W => VirtualKey::W,
            VK_X => VirtualKey::X,
            VK_Y => VirtualKey::Y,
            VK_Z => VirtualKey::Z,
            VK_NUMPAD1 => VirtualKey::Num1,
            VK_NUMPAD2 => VirtualKey::Num2,
            VK_NUMPAD3 => VirtualKey::Num3,
            VK_NUMPAD4 => VirtualKey::Num4,
            VK_NUMPAD5 => VirtualKey::Num5,
            VK_NUMPAD6 => VirtualKey::Num6,
            VK_NUMPAD7 => VirtualKey::Num7,
            VK_NUMPAD8 => VirtualKey::Num8,
            VK_NUMPAD9 => VirtualKey::Num9,
            VK_NUMPAD0 => VirtualKey::Num0,
            VK_ESCAPE => VirtualKey::Esc,
            VK_TAB => VirtualKey::Tab,
            VK_CAPITAL => VirtualKey::CapsLock,
            VK_SHIFT => VirtualKey::Shift,
            VK_CONTROL => VirtualKey::Ctrl,
            VK_MENU => VirtualKey::Alt,
            VK_BACK => VirtualKey::BackSpace,
            VK_RETURN => VirtualKey::Enter,
            VK_SPACE => VirtualKey::Space,
            VK_SNAPSHOT => VirtualKey::PrintScreen,
            VK_SCROLL => VirtualKey::ScrollLock,
            VK_PAUSE => VirtualKey::Pause,
            VK_INSERT => VirtualKey::Insert,
            VK_DELETE => VirtualKey::Delete,
            VK_HOME => VirtualKey::Home,
            VK_END => VirtualKey::End,
            VK_PRIOR => VirtualKey::PageUp,
            VK_NEXT => VirtualKey::PageDown,
            VK_UP => VirtualKey::Up,
            VK_DOWN => VirtualKey::Down,
            VK_LEFT => VirtualKey::Left,
            VK_RIGHT => VirtualKey::Right,
            VK_NUMLOCK => VirtualKey::NumLock,
            VK_ADD => VirtualKey::NumAdd,
            VK_SUBTRACT => VirtualKey::NumSub,
            VK_MULTIPLY => VirtualKey::NumMul,
            VK_DIVIDE => VirtualKey::NumDiv,
            VK_DECIMAL => VirtualKey::NumDecimal,
            VK_F1 => VirtualKey::F1,
            VK_F2 => VirtualKey::F2,
            VK_F3 => VirtualKey::F3,
            VK_F4 => VirtualKey::F4,
            VK_F5 => VirtualKey::F5,
            VK_F6 => VirtualKey::F6,
            VK_F7 => VirtualKey::F7,
            VK_F8 => VirtualKey::F8,
            VK_F9 => VirtualKey::F9,
            VK_F10 => VirtualKey::F10,
            VK_F11 => VirtualKey::F11,
            VK_F12 => VirtualKey::F12,
            VK_F13 => VirtualKey::F13,
            VK_F14 => VirtualKey::F14,
            VK_F15 => VirtualKey::F15,
            VK_F16 => VirtualKey::F16,
            VK_F17 => VirtualKey::F17,
            VK_F18 => VirtualKey::F18,
            VK_F19 => VirtualKey::F19,
            VK_F20 => VirtualKey::F20,
            VK_F21 => VirtualKey::F21,
            VK_F22 => VirtualKey::F22,
            VK_F23 => VirtualKey::F23,
            VK_F24 => VirtualKey::F24,
            _ => VirtualKey::Other(value.0 as _),
        }
    }
}

impl From<VirtualKey> for VIRTUAL_KEY {
    fn from(value: VirtualKey) -> Self {
        match value {
            VirtualKey::Key1 => VK_1,
            VirtualKey::Key2 => VK_2,
            VirtualKey::Key3 => VK_3,
            VirtualKey::Key4 => VK_4,
            VirtualKey::Key5 => VK_5,
            VirtualKey::Key6 => VK_6,
            VirtualKey::Key7 => VK_7,
            VirtualKey::Key8 => VK_8,
            VirtualKey::Key9 => VK_9,
            VirtualKey::Key0 => VK_0,
            VirtualKey::A => VK_A,
            VirtualKey::B => VK_B,
            VirtualKey::C => VK_C,
            VirtualKey::D => VK_D,
            VirtualKey::E => VK_E,
            VirtualKey::F => VK_F,
            VirtualKey::G => VK_G,
            VirtualKey::H => VK_H,
            VirtualKey::I => VK_I,
            VirtualKey::J => VK_J,
            VirtualKey::K => VK_K,
            VirtualKey::L => VK_L,
            VirtualKey::M => VK_M,
            VirtualKey::N => VK_N,
            VirtualKey::O => VK_O,
            VirtualKey::P => VK_P,
            VirtualKey::Q => VK_Q,
            VirtualKey::R => VK_R,
            VirtualKey::S => VK_S,
            VirtualKey::T => VK_T,
            VirtualKey::U => VK_U,
            VirtualKey::V => VK_V,
            VirtualKey::W => VK_W,
            VirtualKey::X => VK_X,
            VirtualKey::Y => VK_Y,
            VirtualKey::Z => VK_Z,
            VirtualKey::Num1 => VK_NUMPAD1,
            VirtualKey::Num2 => VK_NUMPAD2,
            VirtualKey::Num3 => VK_NUMPAD3,
            VirtualKey::Num4 => VK_NUMPAD4,
            VirtualKey::Num5 => VK_NUMPAD5,
            VirtualKey::Num6 => VK_NUMPAD6,
            VirtualKey::Num7 => VK_NUMPAD7,
            VirtualKey::Num8 => VK_NUMPAD8,
            VirtualKey::Num9 => VK_NUMPAD9,
            VirtualKey::Num0 => VK_NUMPAD0,
            VirtualKey::Esc => VK_ESCAPE,
            VirtualKey::Tab => VK_TAB,
            VirtualKey::CapsLock => VK_CAPITAL,
            VirtualKey::Shift => VK_SHIFT,
            VirtualKey::Ctrl => VK_CONTROL,
            VirtualKey::Alt => VK_MENU,
            VirtualKey::BackSpace => VK_BACK,
            VirtualKey::Enter => VK_RETURN,
            VirtualKey::Space => VK_SPACE,
            VirtualKey::PrintScreen => VK_SNAPSHOT,
            VirtualKey::ScrollLock => VK_SCROLL,
            VirtualKey::Pause => VK_PAUSE,
            VirtualKey::Insert => VK_INSERT,
            VirtualKey::Delete => VK_DELETE,
            VirtualKey::Home => VK_HOME,
            VirtualKey::End => VK_END,
            VirtualKey::PageUp => VK_PRIOR,
            VirtualKey::PageDown => VK_NEXT,
            VirtualKey::Up => VK_UP,
            VirtualKey::Down => VK_DOWN,
            VirtualKey::Left => VK_LEFT,
            VirtualKey::Right => VK_RIGHT,
            VirtualKey::NumLock => VK_NUMLOCK,
            VirtualKey::NumAdd => VK_ADD,
            VirtualKey::NumSub => VK_SUBTRACT,
            VirtualKey::NumMul => VK_MULTIPLY,
            VirtualKey::NumDiv => VK_DIVIDE,
            VirtualKey::NumDecimal => VK_DECIMAL,
            VirtualKey::F1 => VK_F1,
            VirtualKey::F2 => VK_F2,
            VirtualKey::F3 => VK_F3,
            VirtualKey::F4 => VK_F4,
            VirtualKey::F5 => VK_F5,
            VirtualKey::F6 => VK_F6,
            VirtualKey::F7 => VK_F7,
            VirtualKey::F8 => VK_F8,
            VirtualKey::F9 => VK_F9,
            VirtualKey::F10 => VK_F10,
            VirtualKey::F11 => VK_F11,
            VirtualKey::F12 => VK_F12,
            VirtualKey::F13 => VK_F13,
            VirtualKey::F14 => VK_F14,
            VirtualKey::F15 => VK_F15,
            VirtualKey::F16 => VK_F16,
            VirtualKey::F17 => VK_F17,
            VirtualKey::F18 => VK_F18,
            VirtualKey::F19 => VK_F19,
            VirtualKey::F20 => VK_F20,
            VirtualKey::F21 => VK_F21,
            VirtualKey::F22 => VK_F22,
            VirtualKey::F23 => VK_F23,
            VirtualKey::F24 => VK_F24,
            VirtualKey::Other(v) => VIRTUAL_KEY(v as _),
        }
    }
}

impl From<VirtualKey> for KeyCode {
    #[inline]
    fn from(vkey: VirtualKey) -> Self {
        unsafe {
            let scan_code = MapVirtualKeyW(VIRTUAL_KEY::from(vkey).0 as u32, MAPVK_VK_TO_VSC);
            Self {
                vkey,
                scan_code: ScanCode(scan_code),
            }
        }
    }
}

impl From<ScanCode> for KeyCode {
    #[inline]
    fn from(scan_code: ScanCode) -> Self {
        unsafe {
            let vkey = VIRTUAL_KEY(MapVirtualKeyW(scan_code.0, MAPVK_VSC_TO_VK) as u16);
            Self {
                vkey: vkey.into(),
                scan_code,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_buttons_contains() {
        let btns = MouseButtons(MouseButton::Left.as_u32());
        assert!(btns.contains(MouseButton::Left));
        assert!(!btns.contains(MouseButton::Right));
    }

    #[test]
    fn mouse_buttons_bitor() {
        let btns = MouseButton::Left | MouseButton::Right;
        assert!(btns.contains(MouseButton::Left));
        assert!(btns.contains(MouseButton::Right));
        assert!(!btns.contains(MouseButton::Middle));
        assert!(!btns.contains(MouseButton::Ex(0)));
        let btns = btns | MouseButton::Middle;
        assert!(btns.contains(MouseButton::Left));
        assert!(btns.contains(MouseButton::Right));
        assert!(btns.contains(MouseButton::Middle));
        assert!(!btns.contains(MouseButton::Ex(0)));
    }
    
    #[test]
    fn array_to_mouse_buttons() {
        let a = [MouseButton::Left, MouseButton::Right];
        let btns = MouseButtons::from(a);
        assert!(btns.contains(MouseButton::Left));
        assert!(btns.contains(MouseButton::Right));
        assert!(!btns.contains(MouseButton::Middle));
        assert!(!btns.contains(MouseButton::Ex(0)));
        let btns = MouseButtons::from(a.as_slice());
        assert!(btns.contains(MouseButton::Left));
        assert!(btns.contains(MouseButton::Right));
        assert!(!btns.contains(MouseButton::Middle));
        assert!(!btns.contains(MouseButton::Ex(0)));
        let v = vec![MouseButton::Left, MouseButton::Right];
        let btns = MouseButtons::from(v);
        assert!(btns.contains(MouseButton::Left));
        assert!(btns.contains(MouseButton::Right));
        assert!(!btns.contains(MouseButton::Middle));
        assert!(!btns.contains(MouseButton::Ex(0)));
        let v = vec![MouseButton::Left, MouseButton::Right];
        let btns = MouseButtons::from(&v);
        assert!(btns.contains(MouseButton::Left));
        assert!(btns.contains(MouseButton::Right));
        assert!(!btns.contains(MouseButton::Middle));
        assert!(!btns.contains(MouseButton::Ex(0)));
    }
}
