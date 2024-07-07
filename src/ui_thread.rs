use crate::*;

use std::os::windows::prelude::*;
use std::sync::{mpsc, Mutex, OnceLock};
use windows::Win32::{
    Foundation::{BOOL, HANDLE, HWND, LPARAM, WPARAM},
    System::Com::{
        CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
    },
    System::Threading::GetThreadId,
    UI::HiDpi::*,
    UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, IsGUIThread, PostThreadMessageW, TranslateMessage, MSG,
        WM_APP,
    },
};

fn enable_dpi_awareness() {
    unsafe {
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).is_ok() {
            log::info!("PerMonitorAwareV2");
            return;
        }
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE).is_ok() {
            log::info!("PerMonitorAware");
            return;
        }
        if SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE).is_ok() {
            log::info!("PerMonitorAware");
            return;
        }
        log::warn!("No changed DPI Awareness");
    }
}

pub(crate) const WM_POST_TASK: u32 = WM_APP;

struct Task(Box<dyn FnOnce() + Send>);

struct Thread {
    th: Option<std::thread::JoinHandle<u32>>,
    task_tx: mpsc::Sender<Task>,
}

impl Thread {
    fn new() -> Self {
        let (task_tx, task_rx) = mpsc::channel::<Task>();
        let (block_tx, block_rx) = mpsc::channel::<()>();
        let th = std::thread::Builder::new()
            .name("wiard UiThread".into())
            .spawn(move || unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).unwrap();
                let _ = IsGUIThread(true);
                Context::init().unwrap();
                block_tx.send(()).ok();
                std::mem::drop(block_tx);
                let mut msg = MSG::default();
                let ret = loop {
                    let ret = GetMessageW(&mut msg, HWND::default(), 0, 0);
                    if ret == BOOL(0) || ret == BOOL(-1) {
                        Context::shutdown();
                        break msg.wParam.0 as u32;
                    }
                    match msg.message {
                        WM_POST_TASK => {
                            let ret = std::panic::catch_unwind(|| {
                                for task in task_rx.try_iter() {
                                    task.0();
                                }
                            });
                            if let Err(e) = ret {
                                Context::send_panic(e);
                                break 1;
                            }
                        }
                        _ => {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                            if let Some(e) = procedure::get_unwind() {
                                Context::send_panic(e);
                                break 1;
                            }
                        }
                    }
                };
                CoUninitialize();
                ret
            })
            .unwrap();
        block_rx.recv().unwrap();
        Self {
            th: Some(th),
            task_tx,
        }
    }

    fn post_message(&self, msg: u32, wparam: WPARAM, lparam: LPARAM) {
        unsafe {
            let th = GetThreadId(HANDLE(self.th.as_ref().unwrap().as_raw_handle()));
            PostThreadMessageW(th, msg, wparam, lparam).ok();
        }
    }

    fn send_task(&self, f: impl FnOnce() + Send + 'static) {
        self.task_tx.send(Task(Box::new(f))).ok();
        self.post_message(WM_POST_TASK, WPARAM(0), LPARAM(0));
    }
}

static THREAD: OnceLock<Mutex<Thread>> = OnceLock::new();

/// Represents UI Thread.
pub struct UiThread;

impl UiThread {
    /// Initializes UI thread.
    ///
    /// In general, no needs to call this function.
    ///
    pub fn init() {
        THREAD.get_or_init(|| {
            enable_dpi_awareness();
            Mutex::new(Thread::new())
        });
    }

    /// Sends a closure to UI thread.
    ///
    /// A sent closure is called in UI thread.
    ///
    /// This function is not wait for calling a closure.
    /// If you want to wait for calling a closure completely, use a chennel such as `mpsc`.
    ///
    #[inline]
    pub fn send_task(f: impl FnOnce() + Send + 'static) {
        THREAD.get().unwrap().lock().unwrap().send_task(f);
    }

    /// Checks if UI thread has finished.
    ///
    /// This function do not block;
    ///
    #[inline]
    pub fn is_finished() -> bool {
        THREAD
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .th
            .as_ref()
            .map_or(true, |th| th.is_finished())
    }

    /// For specifying a receiver to panic when UI thread panics.
    ///
    /// When UI thread catches a panic, the receiver resumes a panic from UI thread.
    ///
    #[inline]
    pub fn set_receiver_for_panic(rx: &impl IsReceiver) {
        Context::set_panic_receiver(rx)
    }

    /// Wait for UI thread to finish.
    #[inline]
    pub fn join() -> std::thread::Result<u32> {
        let th = THREAD.get().unwrap().lock().unwrap().th.take().unwrap();
        th.join()
    }
}
