pub mod ime {
    use crate::*;
    use std::cell::{Cell, OnceCell};
    use windows::core::{ComInterface, IUnknown};
    use windows::Win32::{
        Foundation::{BOOL, HWND, POINT, RECT},
        Globalization::*, System::Com::*, UI::HiDpi::GetDpiForWindow, UI::Input::Ime::*,
        UI::Input::KeyboardAndMouse::GetFocus, UI::TextServices::*,
    };
    pub(crate) struct ImmContext {
        hwnd: HWND,
        himc: HIMC,
        enabled: Cell<bool>,
    }
    impl ImmContext {
        pub fn new(hwnd: HWND) -> Self {
            unsafe {
                let himc = ImmCreateContext();
                ImmAssociateContextEx(hwnd, himc, IACE_CHILDREN);
                Self {
                    hwnd,
                    himc,
                    enabled: Cell::new(true),
                }
            }
        }
        pub fn enable(&self) {
            if !self.enabled.get() {
                unsafe {
                    ImmAssociateContextEx(self.hwnd, self.himc, IACE_CHILDREN);
                }
                self.enabled.set(true);
            }
        }
        pub fn disable(&self) {
            if self.enabled.get() {
                unsafe {
                    ImmAssociateContextEx(self.hwnd, HIMC(0), IACE_IGNORENOCONTEXT);
                }
                self.enabled.set(false);
            }
        }
    }
    impl Drop for ImmContext {
        fn drop(&mut self) {
            unsafe {
                ImmAssociateContextEx(self.hwnd, HIMC(0), IACE_DEFAULT);
                ImmDestroyContext(self.himc);
            }
        }
    }
    pub struct Clause {
        pub range: std::ops::Range<usize>,
        pub targeted: bool,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Clause {
        #[inline]
        fn clone(&self) -> Clause {
            Clause {
                range: ::core::clone::Clone::clone(&self.range),
                targeted: ::core::clone::Clone::clone(&self.targeted),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Clause {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Clause {
        #[inline]
        fn eq(&self, other: &Clause) -> bool {
            self.range == other.range && self.targeted == other.targeted
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralEq for Clause {}
    #[automatically_derived]
    impl ::core::cmp::Eq for Clause {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<std::ops::Range<usize>>;
            let _: ::core::cmp::AssertParamIsEq<bool>;
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Clause {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "Clause",
                "range",
                &self.range,
                "targeted",
                &&self.targeted,
            )
        }
    }
    impl PartialOrd for Clause {
        #[inline]
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            self.range.start.partial_cmp(&other.range.start)
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
        fn composition_string_impl(
            &self,
            param: IME_COMPOSITION_STRING,
        ) -> Option<Vec<u8>> {
            unsafe {
                let byte_len = ImmGetCompositionStringW(self.himc, param, None, 0);
                if byte_len == IMM_ERROR_NODATA || byte_len == IMM_ERROR_GENERAL {
                    return None;
                }
                let mut buf = ::alloc::vec::from_elem(0u8, byte_len as usize);
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
            let s = unsafe {
                let buf = std::slice::from_raw_parts(
                    buf.as_ptr() as *const u16,
                    buf.len() / std::mem::size_of::<u16>(),
                );
                String::from_utf16_lossy(&buf)
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
                let buf = unsafe {
                    std::slice::from_raw_parts(
                        buf.as_ptr() as *const u32,
                        buf.len() / std::mem::size_of::<u32>(),
                    )
                };
                buf.windows(2).map(|a| a[0] as usize..a[1] as usize).collect()
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
            let buf = unsafe {
                std::slice::from_raw_parts(
                    buf.as_ptr() as *const u16,
                    buf.len() / std::mem::size_of::<u16>(),
                )
            };
            let s = String::from_utf16_lossy(&buf);
            (!s.is_empty()).then_some(s)
        }
        pub fn get_cursor_position(&self) -> usize {
            unsafe {
                ImmGetCompositionStringW(self.himc, GCS_CURSORPOS, None, 0) as usize
            }
        }
    }
    impl Drop for Imc {
        fn drop(&mut self) {
            unsafe {
                ImmReleaseContext(self.hwnd, self.himc);
            }
        }
    }
    #[repr(C)]
    struct UiElementSink_Impl {
        identity: *const ::windows::core::IInspectable_Vtbl,
        vtables: (*const <ITfUIElementSink as ::windows::core::Interface>::Vtable,),
        this: UiElementSink,
        count: ::windows::core::imp::WeakRefCount,
    }
    impl UiElementSink_Impl {
        const VTABLES: (<ITfUIElementSink as ::windows::core::Interface>::Vtable,) = (
            <ITfUIElementSink as ::windows::core::Interface>::Vtable::new::<
                Self,
                UiElementSink,
                -1,
            >(),
        );
        const IDENTITY: ::windows::core::IInspectable_Vtbl = ::windows::core::IInspectable_Vtbl::new::<
            Self,
            ITfUIElementSink,
            0,
        >();
        fn new(this: UiElementSink) -> Self {
            Self {
                identity: &Self::IDENTITY,
                vtables: (&Self::VTABLES.0,),
                this,
                count: ::windows::core::imp::WeakRefCount::new(),
            }
        }
    }
    impl ::windows::core::IUnknownImpl for UiElementSink_Impl {
        type Impl = UiElementSink;
        fn get_impl(&self) -> &Self::Impl {
            &self.this
        }
        unsafe fn QueryInterface(
            &self,
            iid: &::windows::core::GUID,
            interface: *mut *const ::core::ffi::c_void,
        ) -> ::windows::core::HRESULT {
            unsafe {
                *interface = if iid
                    == &<::windows::core::IUnknown as ::windows::core::ComInterface>::IID
                    || iid
                        == &<::windows::core::IInspectable as ::windows::core::ComInterface>::IID
                    || iid
                        == &<::windows::core::imp::IAgileObject as ::windows::core::ComInterface>::IID
                {
                    &self.identity as *const _ as *const _
                } else if <ITfUIElementSink as ::windows::core::Interface>::Vtable::matches(
                    iid,
                ) {
                    &self.vtables.0 as *const _ as *const _
                } else {
                    ::core::ptr::null_mut()
                };
                if !(*interface).is_null() {
                    self.count.add_ref();
                    return ::windows::core::HRESULT(0);
                }
                *interface = self.count.query(iid, &self.identity as *const _ as *mut _);
                if (*interface).is_null() {
                    ::windows::core::HRESULT(0x8000_4002)
                } else {
                    ::windows::core::HRESULT(0)
                }
            }
        }
        fn AddRef(&self) -> u32 {
            self.count.add_ref()
        }
        unsafe fn Release(&self) -> u32 {
            let remaining = self.count.release();
            if remaining == 0 {
                unsafe {
                    _ = ::std::boxed::Box::from_raw(self as *const Self as *mut Self);
                }
            }
            remaining
        }
    }
    impl UiElementSink {
        /// Try casting as the provided interface
        ///
        /// # Safety
        ///
        /// This function can only be safely called if `self` has been heap allocated and pinned using
        /// the mechanisms provided by `implement` macro.
        unsafe fn cast<I: ::windows::core::ComInterface>(
            &self,
        ) -> ::windows::core::Result<I> {
            let boxed = (self as *const _ as *const *mut ::core::ffi::c_void).sub(1 + 1)
                as *mut UiElementSink_Impl;
            let mut result = None;
            <UiElementSink_Impl as ::windows::core::IUnknownImpl>::QueryInterface(
                    &*boxed,
                    &I::IID,
                    &mut result as *mut _ as _,
                )
                .and_some(result)
        }
    }
    impl ::core::convert::From<UiElementSink> for ::windows::core::IUnknown {
        fn from(this: UiElementSink) -> Self {
            let this = UiElementSink_Impl::new(this);
            let boxed = ::core::mem::ManuallyDrop::new(::std::boxed::Box::new(this));
            unsafe { ::core::mem::transmute(&boxed.identity) }
        }
    }
    impl ::core::convert::From<UiElementSink> for ::windows::core::IInspectable {
        fn from(this: UiElementSink) -> Self {
            let this = UiElementSink_Impl::new(this);
            let boxed = ::core::mem::ManuallyDrop::new(::std::boxed::Box::new(this));
            unsafe { ::core::mem::transmute(&boxed.identity) }
        }
    }
    impl ::core::convert::From<UiElementSink> for ITfUIElementSink {
        fn from(this: UiElementSink) -> Self {
            let this = UiElementSink_Impl::new(this);
            let mut this = ::core::mem::ManuallyDrop::new(::std::boxed::Box::new(this));
            let vtable_ptr = &this.vtables.0;
            unsafe { ::core::mem::transmute(vtable_ptr) }
        }
    }
    impl ::windows::core::AsImpl<UiElementSink> for ITfUIElementSink {
        unsafe fn as_impl(&self) -> &UiElementSink {
            let this = ::windows::core::Interface::as_raw(self);
            unsafe {
                let this = (this as *mut *mut ::core::ffi::c_void).sub(1 + 0)
                    as *mut UiElementSink_Impl;
                &(*this).this
            }
        }
    }
    struct UiElementSink;
    #[automatically_derived]
    impl ::core::clone::Clone for UiElementSink {
        #[inline]
        fn clone(&self) -> UiElementSink {
            UiElementSink
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for UiElementSink {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "UiElementSink")
        }
    }
    #[allow(non_snake_case)]
    impl ITfUIElementSink_Impl for UiElementSink {
        fn BeginUIElement(
            &self,
            _id: u32,
            show: *mut BOOL,
        ) -> windows::core::Result<()> {
            let hwnd = unsafe { GetFocus() };
            let visibility = Context::get_window_props(
                hwnd,
                |props| props.visible_ime_candidate_window,
            );
            unsafe {
                *show.as_mut().unwrap() = visibility.into();
            }
            {
                ::std::io::_print(format_args!("BeginUIElement\n"));
            };
            Ok(())
        }
        fn UpdateUIElement(&self, _id: u32) -> windows::core::Result<()> {
            {
                ::std::io::_print(format_args!("UpdateUIElement\n"));
            };
            Ok(())
        }
        fn EndUIElement(&self, _id: u32) -> windows::core::Result<()> {
            {
                ::std::io::_print(format_args!("EndUIElement\n"));
            };
            Ok(())
        }
    }
    struct TextService {
        thread_mgr: ITfThreadMgr,
        client_id: u32,
        cookie: u32,
        ui_element: UiElementSink,
    }
    impl Drop for TextService {
        fn drop(&mut self) {
            let source: ITfSource = self.thread_mgr.cast().unwrap();
            unsafe {
                source.UnadviseSink(self.cookie).ok();
                self.thread_mgr.Deactivate().ok();
            }
        }
    }
    const TEXT_SERVICE: ::std::thread::LocalKey<OnceCell<TextService>> = {
        #[inline]
        fn __init() -> OnceCell<TextService> {
            OnceCell::new()
        }
        #[inline]
        unsafe fn __getit(
            init: ::std::option::Option<
                &mut ::std::option::Option<OnceCell<TextService>>,
            >,
        ) -> ::std::option::Option<&'static OnceCell<TextService>> {
            #[thread_local]
            static __KEY: ::std::thread::local_impl::Key<OnceCell<TextService>> = ::std::thread::local_impl::Key::<
                OnceCell<TextService>,
            >::new();
            unsafe {
                __KEY
                    .get(move || {
                        if let ::std::option::Option::Some(init) = init {
                            if let ::std::option::Option::Some(value) = init.take() {
                                return value;
                            } else if true {
                                {
                                    ::core::panicking::panic_fmt(
                                        format_args!(
                                            "internal error: entered unreachable code: {0}",
                                            format_args!("missing default value"),
                                        ),
                                    );
                                };
                            }
                        }
                        __init()
                    })
            }
        }
        unsafe { ::std::thread::LocalKey::new(__getit) }
    };
    pub(crate) fn init_text_service() {
        let thread_mgr: ITfThreadMgr = unsafe {
            CoCreateInstance(&CLSID_TF_ThreadMgr, None, CLSCTX_INPROC_SERVER).unwrap()
        };
        let thread_mgr_ex: ITfThreadMgrEx = thread_mgr.cast().unwrap();
        let client_id = unsafe {
            let mut id = 0;
            if thread_mgr_ex.ActivateEx(&mut id, TF_TMAE_UIELEMENTENABLEDONLY).is_err() {
                return;
            }
            id
        };
        let ui_element_mgr: ITfUIElementMgr = thread_mgr.cast().unwrap();
        let source: ITfSource = ui_element_mgr.cast().unwrap();
        let ui_element = UiElementSink;
        let cookie = unsafe {
            let ui_element: IUnknown = ui_element.cast().unwrap();
            let Ok(cookie) = source.AdviseSink(&ITfUIElementSink::IID, &ui_element) else {
                return;
            };
            cookie
        };
        TEXT_SERVICE
            .with(|tm| {
                tm.get_or_init(move || TextService {
                    thread_mgr,
                    client_id,
                    cookie,
                    ui_element,
                });
            });
    }
}
