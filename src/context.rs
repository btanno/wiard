use crate::*;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{
    atomic::{self, AtomicU64},
    Mutex, OnceLock,
};
use windows::Win32::Foundation::HWND;

pub(crate) struct Object {
    pub event_tx: crate::window::Sender<RecvEventOrPanic>,
    pub props: WindowProps,
}

pub(crate) struct ContextImpl {
    pub window_map: Mutex<HashMap<isize, Object>>,
    pub event_txs: Mutex<HashMap<u64, crate::window::Sender<RecvEventOrPanic>>>,
    panic_receiver: AtomicU64,
}

impl ContextImpl {
    fn new() -> Self {
        Self {
            window_map: Mutex::new(HashMap::new()),
            event_txs: Mutex::new(HashMap::new()),
            panic_receiver: AtomicU64::new(0),
        }
    }
}

static CONTEXT: OnceLock<ContextImpl> = OnceLock::new();

fn get_context() -> &'static ContextImpl {
    CONTEXT.get_or_init(|| ContextImpl::new())
}

pub(crate) struct Context;

impl Context {
    pub fn init() -> Result<()> {
        window::register_class();
        ime::init_text_service();
        Ok(())
    }

    pub fn is_empty() -> bool {
        let window_map = get_context().window_map.lock().unwrap();
        window_map.is_empty()
    }

    pub fn register_window(hwnd: HWND, props: WindowProps, event_rx_id: u64) {
        let ctx = get_context();
        let event_tx = {
            let event_txs = ctx.event_txs.lock().unwrap();
            event_txs.get(&event_rx_id).unwrap().clone()
        };
        let mut window_map = ctx.window_map.lock().unwrap();
        window_map.insert(hwnd.0, Object { props, event_tx });
    }

    pub fn remove_window(hwnd: HWND) -> Option<Object> {
        let mut window_map = get_context().window_map.lock().unwrap();
        window_map.remove(&hwnd.0)
    }

    pub fn close_all_windows() {
        let window_map = get_context().window_map.lock().unwrap();
        for (hwnd, obj) in window_map.iter() {
            obj.event_tx
                .send(RecvEventOrPanic::Event((
                    Event::Closed,
                    Window::from_isize(*hwnd),
                )))
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
            .send(RecvEventOrPanic::Event((event, Window::from_isize(hwnd.0))))
            .ok();
    }

    pub fn get_window_props<F, T>(hwnd: HWND, f: F) -> T
    where
        F: FnOnce(&WindowProps) -> T,
    {
        let window_map = get_context().window_map.lock().unwrap();
        let object = window_map.get(&hwnd.0).unwrap();
        f(&object.props)
    }

    pub fn set_window_props<F>(hwnd: HWND, f: F)
    where
        F: FnOnce(&mut WindowProps),
    {
        let mut window_map = get_context().window_map.lock().unwrap();
        let object = window_map.get_mut(&hwnd.0).unwrap();
        f(&mut object.props)
    }

    pub fn register_event_tx(id: u64, tx: crate::window::Sender<RecvEventOrPanic>) {
        let mut event_txs = get_context().event_txs.lock().unwrap();
        event_txs.insert(id, tx);
    }

    pub fn send_panic(e: Box<dyn Any + Send>) {
        Self::close_all_windows();
        ime::shutdown_text_service();
        let ctx = get_context();
        let mut event_txs = ctx.event_txs.lock().unwrap();
        if let Some(tx) = event_txs.remove(&ctx.panic_receiver.load(atomic::Ordering::SeqCst)) {
            tx.send(RecvEventOrPanic::Panic(e)).ok();
        }
        event_txs.clear();
    }

    pub fn shutdown() {
        Self::close_all_windows();
        ime::shutdown_text_service();
        let mut event_txs = get_context().event_txs.lock().unwrap();
        event_txs.clear();
    }
}
