use crate::*;

use std::cell::RefCell;
use std::os::windows::prelude::*;
use std::sync::{Mutex, OnceLock, mpsc};
use windows::Win32::{
    Foundation::{HANDLE, LPARAM, WPARAM},
    System::Ole::{OleInitialize, OleUninitialize},
    System::Threading::GetThreadId,
    UI::HiDpi::*,
    UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, IsGUIThread, MSG, PostThreadMessageW, TranslateMessage,
    },
};
use windows::core::BOOL;

fn enable_dpi_awareness() {
    unsafe {
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).is_ok() {
            info!("PerMonitorAwareV2");
            return;
        }
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE).is_ok() {
            info!("PerMonitorAware");
            return;
        }
        if SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE).is_ok() {
            info!("PerMonitorAware");
            return;
        }
        warning!("No changed DPI Awareness");
    }
}

struct Task(Box<dyn FnOnce() + Send>);

thread_local! {
    static FINISH_HANDLER: RefCell<Vec<Box<dyn FnOnce() + Send>>> = RefCell::new(vec![]);
}

struct Thread {
    th: Option<std::thread::JoinHandle<u32>>,
    task_tx: mpsc::Sender<Task>,
}

impl Thread {
    fn new<Ts, Ms, Me, Te>(builder: Builder<Ts, Ms, Me, Te>) -> Self
    where
        Ts: FnOnce() + Send + 'static,
        Ms: FnOnce() + Send + 'static,
        Me: FnOnce() + Send + std::panic::UnwindSafe + 'static,
        Te: FnOnce() + Send + 'static,
    {
        let (task_tx, task_rx) = mpsc::channel::<Task>();
        let (block_tx, block_rx) = mpsc::channel::<()>();
        let th = std::thread::Builder::new()
            .name("wiard UiThread".into())
            .spawn(move || unsafe {
                (builder.on_thread_start)();
                let _ = IsGUIThread(true);
                Context::init().unwrap();
                (builder.on_main_loop_start)();
                block_tx.send(()).ok();
                std::mem::drop(block_tx);
                let mut msg = MSG::default();
                let ret = loop {
                    let ret = GetMessageW(&mut msg, None, 0, 0);
                    if ret == BOOL(0) || ret == BOOL(-1) {
                        let ret = std::panic::catch_unwind(|| {
                            (builder.on_main_loop_end)();
                        });
                        if let Err(e) = ret {
                            Context::send_panic(e);
                            break 1;
                        }
                        let finish_handler = FINISH_HANDLER.take();
                        for handler in finish_handler.into_iter() {
                            handler();
                        }
                        Context::cleanup();
                        break msg.wParam.0 as u32;
                    }
                    match msg.message {
                        WM_APP_POST_TASK => {
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
                (builder.on_thread_end)();
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
            if let Some(th) = self.th.as_ref() {
                let th = GetThreadId(HANDLE(th.as_raw_handle()));
                PostThreadMessageW(th, msg, wparam, lparam).ok();
            }
        }
    }

    fn send_task(&self, f: impl FnOnce() + Send + 'static) {
        self.task_tx.send(Task(Box::new(f))).ok();
        self.post_message(WM_APP_POST_TASK, WPARAM(0), LPARAM(0));
    }
}

static THREAD: OnceLock<Mutex<Thread>> = OnceLock::new();

pub struct Builder<Ts = (), Ms = (), Me = (), Te = ()> {
    on_thread_start: Ts,
    on_main_loop_start: Ms,
    on_main_loop_end: Me,
    on_thread_end: Te,
}

impl Builder<(), (), (), ()> {
    pub fn new() -> Builder<
        impl FnOnce() + Send + 'static,
        impl FnOnce() + Send + 'static,
        impl FnOnce() + Send + std::panic::UnwindSafe + 'static,
        impl FnOnce() + Send + 'static,
    > {
        Builder {
            on_thread_start: || unsafe {
                OleInitialize(None).ok();
            },
            on_main_loop_start: || {},
            on_main_loop_end: || {},
            on_thread_end: || unsafe {
                OleUninitialize();
            },
        }
    }
}

impl<Ts, Ms, Me, Te> Builder<Ts, Ms, Me, Te>
where
    Ts: FnOnce() + Send + 'static,
    Ms: FnOnce() + Send + 'static,
    Me: FnOnce() + Send + std::panic::UnwindSafe + 'static,
    Te: FnOnce() + Send + 'static,
{
    /// Executes function `f` after UI thread started.
    pub fn on_thread_start<F>(self, f: F) -> Builder<F, Ms, Me, Te>
    where
        F: FnOnce() + Send + 'static,
    {
        Builder {
            on_thread_start: f,
            on_main_loop_start: self.on_main_loop_start,
            on_main_loop_end: self.on_main_loop_end,
            on_thread_end: self.on_thread_end,
        }
    }

    /// Executes function `f` before the main loop starts in UI thread.
    pub fn on_main_loop_start<F>(self, f: F) -> Builder<Ts, F, Me, Te>
    where
        F: FnOnce() + Send + 'static,
    {
        Builder {
            on_thread_start: self.on_thread_start,
            on_main_loop_start: f,
            on_main_loop_end: self.on_main_loop_end,
            on_thread_end: self.on_thread_end,
        }
    }

    /// Executes function `f` before the main loop ends in UI thread.
    pub fn on_main_loop_end<F>(self, f: F) -> Builder<Ts, Ms, F, Te>
    where
        F: FnOnce() + Send + 'static,
    {
        Builder {
            on_thread_start: self.on_thread_start,
            on_main_loop_start: self.on_main_loop_start,
            on_main_loop_end: f,
            on_thread_end: self.on_thread_end,
        }
    }

    /// Executes function `f` before UI thread ends.
    pub fn on_thread_end<F>(self, f: F) -> Builder<Ts, Ms, Me, F>
    where
        F: FnOnce() + Send + 'static,
    {
        Builder {
            on_thread_start: self.on_thread_start,
            on_main_loop_start: self.on_main_loop_start,
            on_main_loop_end: self.on_main_loop_end,
            on_thread_end: f,
        }
    }

    /// Initializes UI thread.
    pub fn build(self) {
        THREAD.get_or_init(|| {
            enable_dpi_awareness();
            Mutex::new(Thread::new(self))
        });
    }
}

/// Represents UI Thread.
pub struct UiThread;

impl UiThread {
    /// Initializes UI thread.
    ///
    /// In general, no needs to call this function.
    ///
    #[inline]
    pub fn init() {
        Builder::new().build();
    }

    /// Start to build UI thread.
    #[inline]
    pub fn new() -> Builder<
        impl FnOnce() + Send + 'static,
        impl FnOnce() + Send + 'static,
        impl FnOnce() + Send + std::panic::UnwindSafe + 'static,
        impl FnOnce() + Send + 'static,
    > {
        Builder::new()
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
        Self::init();
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
            .is_none_or(|th| th.is_finished())
    }

    /// For specifying a receiver to panic when UI thread panics.
    ///
    /// When UI thread catches a panic, the receiver resumes a panic from UI thread.
    ///
    #[inline]
    pub fn set_receiver_for_panic(rx: &impl IsReceiver) {
        Context::set_panic_receiver(rx)
    }

    #[inline]
    pub fn add_finish_handler(f: impl FnOnce() + Send + 'static) {
        Self::send_task(move || {
            FINISH_HANDLER.with(|handler| {
                handler.borrow_mut().push(Box::new(f));
            });
        });
    }

    /// Wait for UI thread to finish.
    #[inline]
    pub fn join() -> std::thread::Result<u32> {
        let Some(th) = THREAD.get().unwrap().lock().unwrap().th.take() else {
            return Ok(0);
        };
        th.join()
    }
}
