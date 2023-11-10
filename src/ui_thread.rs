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
                unsafe fn shutdown() {
                    Context::shutdown();
                    CoUninitialize();
                }

                CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).unwrap();
                IsGUIThread(true);
                Context::init().unwrap();
                block_tx.send(()).ok();
                std::mem::drop(block_tx);
                let mut msg = MSG::default();
                let ret = loop {
                    let ret = GetMessageW(&mut msg, HWND(0), 0, 0);
                    if ret == BOOL(0) || ret == BOOL(-1) {
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
                                shutdown();
                                std::panic::resume_unwind(e);
                            }
                        }
                        _ => {
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                            if let Some(e) = procedure::get_unwind() {
                                shutdown();
                                std::panic::resume_unwind(e);
                            }
                        }
                    }
                };
                shutdown();
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
            let th = GetThreadId(HANDLE(self.th.as_ref().unwrap().as_raw_handle() as isize));
            PostThreadMessageW(th, msg, wparam, lparam).ok();
        }
    }

    fn send_task(&self, f: impl FnOnce() + Send + 'static) {
        self.task_tx.send(Task(Box::new(f))).ok();
        self.post_message(WM_POST_TASK, WPARAM(0), LPARAM(0));
    }
}

static THREAD: OnceLock<Mutex<Thread>> = OnceLock::new();

pub struct UiThread;

impl UiThread {
    pub fn init() {
        THREAD.get_or_init(|| {
            enable_dpi_awareness();
            Mutex::new(Thread::new())
        });
    }

    #[inline]
    pub fn send_task(f: impl FnOnce() + Send + 'static) {
        THREAD.get().unwrap().lock().unwrap().send_task(f);
    }

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

    #[inline]
    pub fn join() -> std::thread::Result<u32> {
        let th = THREAD.get().unwrap().lock().unwrap().th.take().unwrap();
        th.join()
    }
}
