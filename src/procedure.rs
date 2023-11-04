use crate::*;
use std::any::Any;
use std::cell::RefCell;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

thread_local! {
    static UNWIND: RefCell<Option<Box<dyn Any + Send>>> = RefCell::new(None);
}

fn set_unwind(e: Box<dyn Any + Send>) {
    UNWIND.with_borrow_mut(|unwind| {
        *unwind = Some(e);
    });
}

pub(crate) fn get_unwind() -> Option<Box<dyn Any + Send>> {
    UNWIND.with_borrow_mut(|unwind| unwind.take())
}

unsafe fn on_destroy(hwnd: HWND) -> LRESULT {
    Context::send_event(hwnd, Event::Closed);
    Context::remove_window(hwnd);
    if Context::is_empty() {
        PostQuitMessage(0);
    }
    LRESULT(0)
}

pub(crate) extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ret = std::panic::catch_unwind(|| unsafe {
        match msg {
            WM_DESTROY => on_destroy(hwnd),
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    });
    ret.unwrap_or_else(|e| {
        set_unwind(e);
        LRESULT(0)
    })
}
