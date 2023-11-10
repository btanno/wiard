use crate::*;
use std::cell::RefCell;
use std::path::PathBuf;
use tokio::sync::oneshot;
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::DestroyWindow};

#[derive(Debug)]
pub struct Draw {
    pub position: PhysicalPosition<i32>,
    pub size: PhysicalSize<u32>,
}

#[derive(Debug)]
pub struct Moved {
    pub position: ScreenPosition<i32>,
}

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

#[derive(Debug)]
pub struct Resizing {
    pub size: PhysicalSize<u32>,
    pub edge: ResizingEdge,
}

#[derive(Debug)]
pub struct Resized {
    pub size: PhysicalSize<u32>,
}

#[derive(Debug)]
pub struct MouseInput {
    pub button: MouseButton,
    pub button_state: ButtonState,
    pub mouse_state: MouseState,
}

#[derive(Debug)]
pub struct CursorMoved {
    pub mouse_state: MouseState,
}

#[derive(Debug)]
pub struct CursorEntered {
    pub mouse_state: MouseState,
}

#[derive(Debug)]
pub struct CursorLeft {
    pub mouse_state: MouseState,
}

#[derive(Debug)]
pub struct MouseWheel {
    pub axis: MouseWheelAxis,
    pub distance: i32,
    pub mouse_state: MouseState,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct CharInput {
    pub c: char,
}

#[derive(Debug)]
pub struct ImeBeginComposition {
    position: RefCell<PhysicalPosition<i32>>,
    dpi: i32,
    tx: Option<oneshot::Sender<PhysicalPosition<i32>>>,
}

impl ImeBeginComposition {
    pub(crate) fn new(dpi: i32, tx: oneshot::Sender<PhysicalPosition<i32>>) -> Self {
        Self {
            position: RefCell::new(PhysicalPosition::new(0, 0)),
            dpi,
            tx: Some(tx),
        }
    }

    #[inline]
    pub fn set_position(
        &self,
        position: impl ToPhysical<i32, Output<i32> = PhysicalPosition<i32>>,
    ) {
        *self.position.borrow_mut() = position.to_physical(self.dpi);
    }
}

impl Drop for ImeBeginComposition {
    #[inline]
    fn drop(&mut self) {
        self.tx
            .take()
            .unwrap()
            .send(self.position.borrow().clone())
            .ok();
    }
}

#[derive(Debug)]
pub struct ImeUpdateComposition {
    pub chars: Vec<char>,
    pub clauses: Vec<ime::Clause>,
    pub cursor_position: usize,
}

#[derive(Debug)]
pub struct ImeEndComposition {
    pub result: Option<String>,
}

#[derive(Debug)]
pub struct ImeUpdateCandidateList {
    pub selection: usize,
    pub items: Vec<String>,
}

#[derive(Debug)]
pub struct Maximized {
    pub size: PhysicalSize<u32>,
}

#[derive(Debug)]
pub struct Restored {
    pub size: PhysicalSize<u32>,
}

#[derive(Debug)]
pub struct DpiChanged {
    pub new_dpi: u32,
}

#[derive(Debug)]
pub struct DropFiles {
    pub paths: Vec<PathBuf>,
    pub position: PhysicalPosition<i32>,
}

#[derive(Clone, Debug)]
pub struct CloseRequest {
    hwnd: HWND,
}

impl CloseRequest {
    pub(crate) fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }

    #[inline]
    pub fn destroy(&self) {
        let hwnd = self.hwnd;
        UiThread::send_task(move || unsafe {
            DestroyWindow(hwnd).ok();
        });
    }
}

#[derive(Debug)]
pub struct Other {
    pub msg: u32,
    pub wparam: usize,
    pub lparam: isize,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum Event {
    Activated,
    Inactivate,
    Draw(Draw),
    Moved(Moved),
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
    Minizmized,
    Maximized(Maximized),
    Restored(Restored),
    DpiChanged(DpiChanged),
    DropFiles(DropFiles),
    CloseRequest(CloseRequest),
    Closed,
    Other(Other),
}
