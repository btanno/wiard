use crate::*;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{
    atomic::{self, AtomicU64},
    Mutex, OnceLock,
};
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE},
};

pub(crate) struct Object {
    pub kind: WindowKind,
    pub event_tx: crate::window::Sender<RecvEventOrPanic>,
    pub props: WindowProps,
    pub children: Vec<WindowHandle>,
}

pub(crate) struct ContextImpl {
    pub window_map: Mutex<HashMap<WindowHandle, Object>>,
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
    CONTEXT.get_or_init(ContextImpl::new)
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

    pub fn register_window(kind: WindowKind, props: WindowProps, event_rx_id: u64) {
        let ctx = get_context();
        let event_tx = {
            let event_txs = ctx.event_txs.lock().unwrap();
            if event_txs.is_empty() {
                ctx.panic_receiver
                    .store(event_rx_id, atomic::Ordering::SeqCst);
            }
            event_txs.get(&event_rx_id).unwrap().clone()
        };
        let mut window_map = ctx.window_map.lock().unwrap();
        let window_handle = kind.window_handle();
        let parent = props.parent;
        window_map.insert(
            window_handle,
            Object {
                kind,
                props,
                event_tx,
                children: vec![],
            },
        );
        if let Some(parent) = parent {
            if let Some(parent_obj) = window_map.get_mut(&parent) {
                parent_obj.children.push(window_handle);
            }
        }
    }

    pub fn remove_window(handle: WindowHandle) -> Option<Object> {
        let mut window_map = get_context().window_map.lock().unwrap();
        let obj = window_map.remove(&handle);
        if let Some(obj) = obj.as_ref() {
            for child in &obj.children {
                unsafe {
                    PostMessageW(Some(child.as_hwnd()), WM_CLOSE, WPARAM(0), LPARAM(0)).ok();
                }
            }
        }
        obj
    }

    pub fn close_all_windows() {
        let window_map = get_context().window_map.lock().unwrap();
        for (_, obj) in window_map.iter() {
            obj.event_tx
                .send(RecvEventOrPanic::Event((Event::Closed, obj.kind.clone())))
                .ok();
        }
    }

    pub fn window_is_none(handle: WindowHandle) -> bool {
        let window_map = get_context().window_map.lock().unwrap();
        !window_map.contains_key(&handle)
    }

    pub fn send_event(handle: WindowHandle, event: Event) {
        let window_map = get_context().window_map.lock().unwrap();
        let Some(object) = window_map.get(&handle) else {
            return;
        };
        object
            .event_tx
            .send(RecvEventOrPanic::Event((event, object.kind.clone())))
            .ok();
    }

    pub fn get_window_props<F, T>(handle: WindowHandle, f: F) -> Option<T>
    where
        F: FnOnce(&WindowProps) -> T,
    {
        let window_map = get_context().window_map.lock().unwrap();
        let object = window_map.get(&handle)?;
        Some(f(&object.props))
    }

    pub fn set_window_props<F>(handle: WindowHandle, f: F)
    where
        F: FnOnce(&mut WindowProps),
    {
        let mut window_map = get_context().window_map.lock().unwrap();
        let object = window_map.get_mut(&handle).unwrap();
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

    pub fn set_panic_receiver(rx: &impl IsReceiver) {
        get_context()
            .panic_receiver
            .store(rx.id(), atomic::Ordering::SeqCst);
    }

    pub fn shutdown() {
        Self::close_all_windows();
        ime::shutdown_text_service();
        let mut event_txs = get_context().event_txs.lock().unwrap();
        event_txs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_is_none() {
        assert!(Context::window_is_none(WindowHandle::default()));
    }
}
