use crate::*;
use std::cell::{Cell, OnceCell, RefCell};
use windows::Win32::{
    Foundation::{BOOL, HWND, POINT, RECT},
    System::Com::*,
    UI::Input::Ime::*,
    UI::Input::KeyboardAndMouse::GetFocus,
    UI::TextServices::*,
};
use windows::core::Interface;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct ImcHandle(isize);

impl ImcHandle {
    fn new(himc: HIMC) -> Self {
        Self(himc.0 as isize)
    }

    fn as_himc(&self) -> HIMC {
        HIMC(self.0 as *mut _)
    }
}

pub(crate) struct ImmContext {
    window_handle: WindowHandle,
    imc_handle: ImcHandle,
    enabled: Cell<bool>,
}

impl ImmContext {
    pub fn new(window_handle: WindowHandle) -> Self {
        unsafe {
            let himc = ImmCreateContext();
            let _ = ImmAssociateContextEx(window_handle.as_hwnd(), himc, IACE_CHILDREN);
            Self {
                window_handle,
                imc_handle: ImcHandle::new(himc),
                enabled: Cell::new(true),
            }
        }
    }

    pub fn enable(&self) {
        if !self.enabled.get() {
            unsafe {
                let _ = ImmAssociateContextEx(
                    self.window_handle.as_hwnd(),
                    self.imc_handle.as_himc(),
                    IACE_CHILDREN,
                );
            }
            self.enabled.set(true);
        }
    }

    pub fn disable(&self) {
        if self.enabled.get() {
            unsafe {
                let _ = ImmAssociateContextEx(
                    self.window_handle.as_hwnd(),
                    HIMC::default(),
                    IACE_IGNORENOCONTEXT,
                );
            }
            self.enabled.set(false);
        }
    }
}

impl Drop for ImmContext {
    fn drop(&mut self) {
        unsafe {
            let _ =
                ImmAssociateContextEx(self.window_handle.as_hwnd(), HIMC::default(), IACE_DEFAULT);
            let _ = ImmDestroyContext(self.imc_handle.as_himc());
        }
    }
}

/// Represents a clause when an IME composition.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Clause {
    pub range: std::ops::Range<usize>,
    pub targeted: bool,
}

impl PartialOrd for Clause {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Clause {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.range.start.cmp(&other.range.start)
    }
}

pub(crate) struct Imc {
    hwnd: HWND,
    himc: HIMC,
}

impl Imc {
    pub fn get(hwnd: HWND) -> Self {
        let himc = unsafe { ImmGetContext(hwnd) };
        Self { hwnd, himc }
    }

    pub fn set_candidate_window_position(
        &self,
        position: PhysicalPosition<i32>,
        enable_exclude_rect: bool,
    ) {
        let position: POINT = position.into();
        let form = CANDIDATEFORM {
            dwStyle: CFS_CANDIDATEPOS,
            dwIndex: 0,
            ptCurrentPos: position,
            ..Default::default()
        };
        unsafe {
            let _ = ImmSetCandidateWindow(self.himc, &form);
        }
        if !enable_exclude_rect {
            let form = CANDIDATEFORM {
                dwStyle: CFS_EXCLUDE,
                dwIndex: 0,
                ptCurrentPos: position,
                rcArea: RECT {
                    left: position.x,
                    top: position.y,
                    right: position.x,
                    bottom: position.y,
                },
            };
            unsafe {
                let _ = ImmSetCandidateWindow(self.himc, &form);
            }
        }
    }

    fn composition_string_impl(&self, param: IME_COMPOSITION_STRING) -> Option<Vec<u8>> {
        unsafe {
            let byte_len = ImmGetCompositionStringW(self.himc, param, None, 0);
            if byte_len == IMM_ERROR_NODATA || byte_len == IMM_ERROR_GENERAL {
                return None;
            }
            let mut buf = vec![0u8; byte_len as usize];
            ImmGetCompositionStringW(
                self.himc,
                param,
                Some(buf.as_mut_ptr() as *mut std::ffi::c_void),
                byte_len as u32,
            );
            Some(buf)
        }
    }

    pub fn get_composition_string(&self) -> Option<String> {
        let buf = self.composition_string_impl(GCS_COMPSTR)?;
        if buf.is_empty() {
            return None;
        }
        let s = unsafe {
            let buf = std::slice::from_raw_parts(
                buf.as_ptr() as *const u16,
                buf.len() / std::mem::size_of::<u16>(),
            );
            String::from_utf16_lossy(buf)
        };
        (!s.is_empty()).then_some(s)
    }

    pub fn get_composition_clauses(&self) -> Option<Vec<Clause>> {
        let targets: Vec<bool> = self
            .composition_string_impl(GCS_COMPATTR)?
            .into_iter()
            .map(|a| a as u32 == ATTR_TARGET_CONVERTED)
            .collect();
        let clauses: Vec<std::ops::Range<usize>> = {
            let buf = self.composition_string_impl(GCS_COMPCLAUSE)?;
            if buf.is_empty() {
                return None;
            }
            let buf = unsafe {
                std::slice::from_raw_parts(
                    buf.as_ptr() as *const u32,
                    buf.len() / std::mem::size_of::<u32>(),
                )
            };
            buf.windows(2)
                .map(|a| a[0] as usize..a[1] as usize)
                .collect()
        };
        Some(
            clauses
                .into_iter()
                .map(|r| Clause {
                    targeted: targets[r.start],
                    range: r,
                })
                .collect(),
        )
    }

    pub fn get_composition_result(&self) -> Option<String> {
        let buf = self.composition_string_impl(GCS_RESULTSTR)?;
        if buf.is_empty() {
            return None;
        }
        let buf = unsafe {
            std::slice::from_raw_parts(
                buf.as_ptr() as *const u16,
                buf.len() / std::mem::size_of::<u16>(),
            )
        };
        let s = String::from_utf16_lossy(buf);
        (!s.is_empty()).then_some(s)
    }

    pub fn get_cursor_position(&self) -> usize {
        unsafe { ImmGetCompositionStringW(self.himc, GCS_CURSORPOS, None, 0) as usize }
    }
}

impl Drop for Imc {
    fn drop(&mut self) {
        unsafe {
            let _ = ImmReleaseContext(self.hwnd, self.himc);
        }
    }
}

struct TextService {
    thread_mgr: RefCell<Option<ITfThreadMgr>>,
    cookie: u32,
}

impl TextService {
    fn shutdown(&self) {
        let thread_mgr = self.thread_mgr.take().unwrap();
        let source: ITfSource = thread_mgr.cast().unwrap();
        unsafe {
            source.UnadviseSink(self.cookie).ok();
            thread_mgr.Deactivate().ok();
        }
    }
}

thread_local! {
    static TEXT_SERVICE: OnceCell<TextService> = const { OnceCell::new() };
}

fn thread_mgr() -> ITfThreadMgr {
    TEXT_SERVICE.with(|ts| {
        ts.get()
            .unwrap()
            .thread_mgr
            .borrow()
            .as_ref()
            .unwrap()
            .clone()
    })
}

fn ui_element_mgr() -> ITfUIElementMgr {
    thread_mgr().cast().unwrap()
}

#[windows_core::implement(ITfUIElementSink)]
struct UiElementSink;

impl UiElementSink {
    #[allow(clippy::new_ret_no_self)]
    fn new() -> ITfUIElementSink {
        Self.into()
    }
}

#[allow(non_snake_case)]
impl ITfUIElementSink_Impl for UiElementSink_Impl {
    fn BeginUIElement(&self, _id: u32, show: *mut BOOL) -> windows::core::Result<()> {
        let hwnd = unsafe { GetFocus() };
        if hwnd.is_invalid() {
            return Ok(());
        }
        let visibility = Context::get_window_props(WindowHandle::new(hwnd), |props| {
            props.visible_ime_candidate_window
        })
        .unwrap_or(true);
        unsafe {
            *show.as_mut().unwrap() = visibility.into();
        }
        if visibility {
            return Ok(());
        }
        Ok(())
    }

    fn UpdateUIElement(&self, id: u32) -> windows::core::Result<()> {
        let hwnd = unsafe { GetFocus() };
        if hwnd == HWND::default() {
            return Ok(());
        }
        unsafe {
            let ui_element = ui_element_mgr().GetUIElement(id)?;
            let candidate_list: ITfCandidateListUIElement = ui_element.cast().unwrap();
            let count = candidate_list.GetCount()?;
            let selection = candidate_list.GetSelection()? as usize;
            let items = (0..count)
                .map(|i| candidate_list.GetString(i).map(|s| s.to_string()))
                .collect::<windows::core::Result<Vec<_>>>()?;
            Context::send_event(
                WindowHandle::new(hwnd),
                Event::ImeUpdateCandidateList(event::ImeUpdateCandidateList { selection, items }),
            );
        }
        Ok(())
    }

    fn EndUIElement(&self, _id: u32) -> windows::core::Result<()> {
        let hwnd = unsafe { GetFocus() };
        if hwnd == HWND::default() {
            return Ok(());
        }
        let handle = WindowHandle::new(hwnd);
        let visibility =
            Context::get_window_props(handle, |props| props.visible_ime_candidate_window)
                .unwrap_or(true);
        if !visibility {
            Context::send_event(handle, Event::ImeEndCandidateList);
        }
        Ok(())
    }
}

pub(crate) fn init_text_service() {
    let thread_mgr: ITfThreadMgr =
        unsafe { CoCreateInstance(&CLSID_TF_ThreadMgr, None, CLSCTX_INPROC_SERVER).unwrap() };
    let thread_mgr_ex: ITfThreadMgrEx = thread_mgr.cast().unwrap();
    unsafe {
        let mut id = 0;
        let ret = thread_mgr_ex.ActivateEx(&mut id, TF_TMAE_UIELEMENTENABLEDONLY);
        if ret.is_err() {
            return;
        }
    }
    let ui_element_mgr: ITfUIElementMgr = thread_mgr.cast().unwrap();
    let source: ITfSource = ui_element_mgr.cast().unwrap();
    let cookie = unsafe {
        let ui_element = UiElementSink::new();
        let Ok(cookie) = source.AdviseSink(&ITfUIElementSink::IID, &ui_element) else {
            return;
        };
        cookie
    };
    TEXT_SERVICE.with(|tm| {
        tm.get_or_init(move || TextService {
            thread_mgr: RefCell::new(Some(thread_mgr)),
            cookie,
        });
    });
}

pub(crate) fn shutdown_text_service() {
    TEXT_SERVICE.with(|tm| tm.get().unwrap().shutdown());
}
