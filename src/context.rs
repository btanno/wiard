use crate::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use windows::Win32::Foundation::HWND;

pub(crate) struct Object {
    pub event_tx: crate::window::Sender<RecvEvent>,
}

struct ContextImpl {
    window_map: HashMap<isize, Object>,
}

impl ContextImpl {
    fn new() -> Self {
        Self {
            window_map: HashMap::new(),
        }
    }
}

static CONTEXT: OnceLock<Mutex<ContextImpl>> = OnceLock::new();

fn get_context() -> &'static Mutex<ContextImpl> {
    CONTEXT.get_or_init(|| Mutex::new(ContextImpl::new()))
}

pub(crate) struct Context;

impl Context {
    pub fn is_empty() -> bool {
        let ctx = get_context().lock().unwrap();
        ctx.window_map.is_empty()
    }

    pub fn register_window(hwnd: HWND, event_tx: crate::window::Sender<RecvEvent>) {
        let mut ctx = get_context().lock().unwrap();
        ctx.window_map.insert(hwnd.0, Object { event_tx });
    }

    pub fn remove_window(hwnd: HWND) -> Option<Object> {
        let mut ctx = get_context().lock().unwrap();
        ctx.window_map.remove(&hwnd.0)
    }

    pub fn close_all_windows() {
        let ctx = get_context().lock().unwrap();
        for (hwnd, obj) in ctx.window_map.iter() {
            obj.event_tx.send((Event::Closed, Window::from_isize(*hwnd))).ok();
        }
    }

    pub fn window_is_none(hwnd: HWND) -> bool {
        let ctx = get_context().lock().unwrap();
        !ctx.window_map.contains_key(&hwnd.0)
    }

    pub fn send_event(hwnd: HWND, event: Event) {
        let ctx = get_context().lock().unwrap();
        let Some(object) = ctx.window_map.get(&hwnd.0) else { return; };
        object.event_tx.send((event, Window::from_isize(hwnd.0))).ok();
    }
}
