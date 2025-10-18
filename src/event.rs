use crate::*;
use std::cell::Cell;
use std::path::PathBuf;
use tokio::sync::oneshot;
use windows::Win32::{Foundation::LPARAM, UI::WindowsAndMessaging::*};

/// An event when a window request to draw.
#[derive(Clone, Debug)]
pub struct Draw {
    pub invalidate_rect: PhysicalRect<i32>,
}

/// An event when window moved.
#[derive(Clone, Debug)]
pub struct Moved {
    pub position: ScreenPosition<i32>,
}

/// An moving edge of window when resizing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResizingEdge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// An event when resizing a window;
#[derive(Clone, Debug)]
pub struct Resizing {
    pub size: PhysicalSize<u32>,
    pub edge: ResizingEdge,
}

/// An event when resized or restored a window from maximized.
#[derive(Clone, Debug)]
pub struct Resized {
    pub size: PhysicalSize<u32>,
}

/// An event when a mouse button pressed and released.
#[derive(Clone, Debug)]
pub struct MouseInput {
    pub button: MouseButton,
    pub button_state: ButtonState,
    pub mouse_state: MouseState,
}

/// An event when a mouse cursor moved.
#[derive(Clone, Debug)]
pub struct CursorMoved {
    pub mouse_state: MouseState,
}

/// An event when a mouse cursor entered a window.
#[derive(Clone, Debug)]
pub struct CursorEntered {
    pub mouse_state: MouseState,
}

/// An event when a mouse cursor left a window.
#[derive(Debug)]
pub struct CursorLeft {
    pub mouse_state: MouseState,
}

// An event when a mouse wheel is rotated.
#[derive(Clone, Debug)]
pub struct MouseWheel {
    pub axis: MouseWheelAxis,
    pub distance: i32,
    pub mouse_state: MouseState,
}

/// An event when keyboard is input.
#[derive(Clone, Debug)]
pub struct KeyInput {
    pub key_code: KeyCode,
    pub key_state: KeyState,
    pub prev_pressed: bool,
}

impl KeyInput {
    #[inline]
    pub fn is(&self, key_code: impl Into<KeyCode>, key_state: KeyState) -> bool {
        self.key_code == key_code.into() && self.key_state == key_state
    }
}

/// An event that receive a keyboard input as the charcter code.
#[derive(Clone, Debug)]
pub struct CharInput {
    pub c: char,
}

/// An event of beginning IME composition.
///
/// When this event is dropped, this event send an IME candidate window position to the window.
/// Therefore, UiThread wait until this event is dropped.
///
#[derive(Debug)]
pub struct ImeBeginComposition {
    position: Cell<PhysicalPosition<i32>>,
    dpi: i32,
    tx: Option<oneshot::Sender<PhysicalPosition<i32>>>,
}

impl ImeBeginComposition {
    pub(crate) fn new(dpi: i32, tx: oneshot::Sender<PhysicalPosition<i32>>) -> Self {
        Self {
            position: Cell::new(PhysicalPosition::new(0, 0)),
            dpi,
            tx: Some(tx),
        }
    }

    #[inline]
    pub fn set_position(
        &self,
        position: impl ToPhysical<i32, Output<i32> = PhysicalPosition<i32>>,
    ) {
        self.position.set(position.to_physical(self.dpi));
    }
}

impl Drop for ImeBeginComposition {
    #[inline]
    fn drop(&mut self) {
        self.tx.take().unwrap().send(self.position.get()).ok();
    }
}

/// An event when IME composition is updated.
#[derive(Clone, Debug)]
pub struct ImeUpdateComposition {
    pub chars: Vec<char>,
    pub clauses: Vec<ime::Clause>,
    pub cursor_position: usize,
}

/// An event when IME composition is finished.
#[derive(Clone, Debug)]
pub struct ImeEndComposition {
    pub result: Option<String>,
}

/// An event when IME candidate list is updated.
#[derive(Clone, Debug)]
pub struct ImeUpdateCandidateList {
    pub selection: usize,
    pub items: Vec<String>,
}

/// An event of maximized a window.
#[derive(Clone, Debug)]
pub struct Maximized {
    pub size: PhysicalSize<u32>,
}

/// An event of restored a window from minimized.
#[derive(Clone, Debug)]
pub struct Restored {
    pub size: PhysicalSize<u32>,
}

/// An event of changed DPI.
#[derive(Clone, Debug)]
pub struct DpiChanged {
    pub new_dpi: u32,
}

/// An event of dropped files.
#[derive(Clone, Debug)]
pub struct DropFiles {
    pub paths: Vec<PathBuf>,
    pub position: PhysicalPosition<i32>,
}

/// Values which can return from WM_NCHITTEST
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum NcHitTestValue {
    Border = HTBORDER,
    Bottom = HTBOTTOM,
    BottomLeft = HTBOTTOMLEFT,
    BottomRight = HTBOTTOMRIGHT,
    Left = HTLEFT,
    Right = HTRIGHT,
    Top = HTTOP,
    TopLeft = HTTOPLEFT,
    TopRight = HTTOPRIGHT,
    Caption = HTCAPTION,
    Client = HTCLIENT,
    Size = HTSIZE,
    Help = HTHELP,
    HScroll = HTHSCROLL,
    VScroll = HTVSCROLL,
    Menu = HTMENU,
    MaxButton = HTMAXBUTTON,
    MinButton = HTMINBUTTON,
    CloseButton = HTCLOSE,
    SysMenu = HTSYSMENU,
    Error = HTERROR as u32,
    Transparent = HTTRANSPARENT as u32,
}

/// An event of non client area hit test.
///
/// UiThread wait until this event is dropped.
///
#[derive(Debug)]
pub struct NcHitTest {
    pub position: PhysicalPosition<i32>,
    value: Cell<Option<NcHitTestValue>>,
    tx: Option<oneshot::Sender<Option<NcHitTestValue>>>,
}

impl NcHitTest {
    pub(crate) fn new(lparam: LPARAM, tx: oneshot::Sender<Option<NcHitTestValue>>) -> Self {
        let position = lparam_to_point(lparam);
        Self {
            position,
            value: Cell::new(None),
            tx: Some(tx),
        }
    }

    #[inline]
    pub fn get(&self) -> Option<NcHitTestValue> {
        self.value.get()
    }

    #[inline]
    pub fn set(&self, value: Option<NcHitTestValue>) {
        self.value.set(value);
    }
}

impl Drop for NcHitTest {
    fn drop(&mut self) {
        self.tx.take().unwrap().send(self.value.get()).ok();
    }
}

/// An event of occurred at the notify icon.
#[derive(Clone, Debug)]
pub struct NotifyIcon {
    pub id: super::NotifyIcon,
    pub event: NotifyIconEvent,
}

impl PartialEq<super::NotifyIcon> for NotifyIcon {
    #[inline]
    fn eq(&self, other: &super::NotifyIcon) -> bool {
        &self.id == other
    }
}

impl PartialEq<NotifyIcon> for super::NotifyIcon {
    #[inline]
    fn eq(&self, other: &NotifyIcon) -> bool {
        self == &other.id
    }
}

/// An event of pushed the menu item.
#[derive(Clone, Debug)]
pub struct MenuCommand {
    pub index: usize,
    pub handle: MenuHandle,
}

/// An event that requests to show context menu.
#[derive(Clone, Debug)]
pub struct ContextMenu {
    pub clicked_window: WindowHandle,
    pub position: ScreenPosition<i32>,
}

/// An event of changed the color mode.
#[derive(Clone, Debug)]
pub struct ColorModeChanged {
    pub current: ColorModeState,
    pub previous: ColorModeState,
}

/// An event of request to close the window.
///
/// This event is called when the window is set `false` to [`auto_close()`].
///
/// [`auto_close()`]: ../struct.WindowBuilder.html#method.auto_close
///
#[derive(Clone, Debug)]
pub struct CloseRequest {
    handle: WindowHandle,
}

impl CloseRequest {
    pub(crate) fn new(handle: WindowHandle) -> Self {
        Self { handle }
    }

    #[inline]
    pub fn destroy(&self) {
        let handle = self.handle;
        UiThread::send_task(move || unsafe {
            DestroyWindow(handle.as_hwnd()).ok();
        });
    }
}

/// An event which defined by an user.
#[derive(Clone, Debug)]
pub struct App {
    pub index: u32,
    pub value0: usize,
    pub value1: isize,
}

impl App {
    #[inline]
    pub fn new(index: u32, value0: usize, value1: isize) -> Self {
        Self {
            index,
            value0,
            value1,
        }
    }
}

/// Other window messages
#[derive(Clone, Debug)]
pub struct Other {
    pub msg: u32,
    pub wparam: usize,
    pub lparam: isize,
}

/// Represents a event.
#[derive(Debug)]
#[non_exhaustive]
pub enum Event {
    Activated,
    Inactivate,
    Draw(Draw),
    Moved(Moved),
    EnterResizing,
    Resizing(Resizing),
    Resized(Resized),
    MouseInput(MouseInput),
    CursorMoved(CursorMoved),
    CursorEntered(CursorEntered),
    CursorLeft(CursorLeft),
    MouseWheel(MouseWheel),
    KeyInput(KeyInput),
    CharInput(CharInput),
    ImeBeginComposition(ImeBeginComposition),
    ImeUpdateComposition(ImeUpdateComposition),
    ImeEndComposition(ImeEndComposition),
    ImeBeginCandidateList,
    ImeUpdateCandidateList(ImeUpdateCandidateList),
    ImeEndCandidateList,
    MenuCommand(MenuCommand),
    ContextMenu(ContextMenu),
    Minizmized,
    Maximized(Maximized),
    Restored(Restored),
    DpiChanged(DpiChanged),
    DropFiles(DropFiles),
    NcHitTest(NcHitTest),
    NotifyIcon(NotifyIcon),
    ColorModeChanged(ColorModeChanged),
    CloseRequest(CloseRequest),
    Closed,
    App(App),
    Other(Other),
}
