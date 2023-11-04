use crate::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use windows::Win32::Foundation::HWND;

pub(crate) struct Object {
    pub event_tx: crate::window::Sender<RecvEvent>,
}

struct ContextImpl {
    window_map: Mutex<HashMap<isize, Object>>,
    event_txs: Mutex<HashMap<u64, crate::window::Sender<RecvEvent>>>,
}

impl ContextImpl {
    fn new() -> Self {
        Self {
            window_map: Mutex::new(HashMap::new()),
            event_txs: Mutex::new(HashMap::new()),
        }
    }
}

static CONTEXT: OnceLock<ContextImpl> = OnceLock::new();

fn get_context() -> &'static ContextImpl {
    CONTEXT.get_or_init(|| ContextImpl::new())
}

pub(crate) struct Context;

impl Context {
    pub fn is_empty() -> bool {
        let window_map = get_context().window_map.lock().unwrap();
        window_map.is_empty()
    }

    pub fn register_window(hwnd: HWND, event_rx_id: u64) {
        let ctx = get_context();
        let event_tx = {
            let mut event_txs = ctx.event_txs.lock().unwrap();
            event_txs.get(&event_rx_id).unwrap().clone()
        };
        let mut window_map = ctx.window_map.lock().unwrap();
        window_map.insert(hwnd.0, Object { event_tx });
    }

    pub fn remove_window(hwnd: HWND) -> Option<Object> {
        let mut window_map = get_context().window_map.lock().unwrap();
        window_map.remove(&hwnd.0)
    }

    pub fn close_all_windows() {
        let window_map = get_context().window_map.lock().unwrap();
        for (hwnd, obj) in window_map.iter() {
            obj.event_tx
                .send((Event::Closed, Window::from_isize(*hwnd)))
                .ok();
        }
    }

    pub fn window_is_none(hwnd: HWND) -> bool {
        let window_map = get_context().window_map.lock().unwrap();
        window_map.contains_key(&hwnd.0)
    }

    pub fn send_event(hwnd: HWND, event: Event) {
        let window_map = get_context().window_map.lock().unwrap();
        let Some(object) = window_map.get(&hwnd.0) else {
            return;
        };
        object
            .event_tx
            .send((event, Window::from_isize(hwnd.0)))
            .ok();
    }

    pub fn register_event_tx(id: u64, tx: crate::window::Sender<RecvEvent>) {
        let mut event_txs = get_context().event_txs.lock().unwrap();
        event_txs.insert(id, tx);
    }

    pub fn shutdown() {
        Self::close_all_windows();
        let mut event_txs = get_context().event_txs.lock().unwrap();
        event_txs.clear();
    }
}
