use crate::*;
use std::any::Any;
use std::sync::atomic::{self, AtomicU64};
use tokio::sync::oneshot;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, WPARAM},
    Graphics::Gdi::{GetStockObject, HBRUSH, WHITE_BRUSH},
    System::LibraryLoader::GetModuleHandleW,
    UI::HiDpi::GetDpiForWindow,
    UI::Shell::DragAcceptFiles,
    UI::WindowsAndMessaging::*,
};

const WINDOW_CLASS_NAME: PCWSTR = windows::core::w!("wiard_window_class");

pub(crate) fn register_class() {
    unsafe {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_VREDRAW | CS_HREDRAW,
            lpfnWndProc: Some(procedure::window_proc),
            hInstance: HINSTANCE(GetModuleHandleW(None).unwrap().0),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            lpszClassName: WINDOW_CLASS_NAME,
            hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 {
            panic!("RegisterClassExW failed");
        }
    }
}

type Receiver<T> = tokio::sync::mpsc::UnboundedReceiver<T>;
pub(crate) type Sender<T> = tokio::sync::mpsc::UnboundedSender<T>;

pub(crate) enum RecvEventOrPanic {
    Event(RecvEvent),
    Panic(Box<dyn Any + Send>),
}

pub type RecvEvent = (Event, Window);
pub type AsyncRecvEvent = (Event, AsyncWindow);

fn gen_id() -> u64 {
    static ID: AtomicU64 = AtomicU64::new(0);
    ID.fetch_add(1, atomic::Ordering::SeqCst)
}

pub trait IsReceiver {
    fn id(&self) -> u64;
}

pub struct EventReceiver {
    id: u64,
    rx: Receiver<RecvEventOrPanic>,
}

impl EventReceiver {
    #[inline]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let id = gen_id();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Context::register_event_tx(id, tx);
        Self { id, rx }
    }

    #[inline]
    pub fn recv(&mut self) -> Option<RecvEvent> {
        match self.rx.blocking_recv()? {
            RecvEventOrPanic::Event(re) => Some(re),
            RecvEventOrPanic::Panic(e) => std::panic::resume_unwind(e),
        }
    }

    #[inline]
    pub fn try_recv(&mut self) -> TryRecvResult<RecvEvent> {
        use tokio::sync::mpsc;

        match self.rx.try_recv() {
            Ok(ret) => match ret {
                RecvEventOrPanic::Event(re) => Ok(re),
                RecvEventOrPanic::Panic(e) => std::panic::resume_unwind(e),
            },
            Err(mpsc::error::TryRecvError::Empty) => Err(TryRecvError::Empty),
            Err(mpsc::error::TryRecvError::Disconnected) => Err(TryRecvError::Disconnected),
        }
    }
}

impl IsReceiver for EventReceiver {
    #[inline]
    fn id(&self) -> u64 {
        self.id
    }
}

pub struct AsyncEventReceiver {
    id: u64,
    rx: Receiver<RecvEventOrPanic>,
}

impl AsyncEventReceiver {
    #[inline]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let id = gen_id();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Context::register_event_tx(id, tx);
        Self { id, rx }
    }

    #[inline]
    pub async fn recv(&mut self) -> Option<AsyncRecvEvent> {
        let ret = match self.rx.recv().await? {
            RecvEventOrPanic::Event(re) => re,
            RecvEventOrPanic::Panic(e) => std::panic::resume_unwind(e),
        };
        Some((ret.0, AsyncWindow { hwnd: ret.1.hwnd }))
    }

    #[inline]
    pub fn try_recv(&mut self) -> TryRecvResult<AsyncRecvEvent> {
        use tokio::sync::mpsc;

        match self.rx.try_recv() {
            Ok(ret) => {
                let ret = match ret {
                    RecvEventOrPanic::Event(re) => re,
                    RecvEventOrPanic::Panic(e) => std::panic::resume_unwind(e),
                };
                Ok((ret.0, AsyncWindow { hwnd: ret.1.hwnd }))
            }
            Err(mpsc::error::TryRecvError::Empty) => Err(TryRecvError::Empty),
            Err(mpsc::error::TryRecvError::Disconnected) => Err(TryRecvError::Disconnected),
        }
    }
}

impl IsReceiver for AsyncEventReceiver {
    #[inline]
    fn id(&self) -> u64 {
        self.id
    }
}

pub struct WindowBuilder<'a, Rx, Title = &'static str, Sz = LogicalSize<u32>, Sty = WindowStyle> {
    event_rx: &'a Rx,
    title: Title,
    inner_size: Sz,
    style: Sty,
    visibility: bool,
    enable_ime: bool,
    visible_ime_candidate_window: bool,
    accept_drop_files: bool,
    auto_close: bool,
}

impl<'a, Rx> WindowBuilder<'a, Rx> {
    #[inline]
    pub fn new(event_rx: &'a Rx) -> Self {
        UiThread::init();
        Self {
            event_rx,
            title: "",
            inner_size: LogicalSize::new(1024, 768),
            style: WindowStyle::default(),
            visibility: true,
            enable_ime: true,
            visible_ime_candidate_window: true,
            accept_drop_files: false,
            auto_close: true,
        }
    }
}

impl<'a, Rx, Title, Sz, Sty> WindowBuilder<'a, Rx, Title, Sz, Sty> {
    #[inline]
    pub fn title<T>(self, title: T) -> WindowBuilder<'a, Rx, T, Sz, Sty>
    where
        T: Into<String>,
    {
        WindowBuilder {
            event_rx: self.event_rx,
            title,
            inner_size: self.inner_size,
            style: self.style,
            visibility: self.visibility,
            enable_ime: self.enable_ime,
            visible_ime_candidate_window: self.visible_ime_candidate_window,
            accept_drop_files: self.accept_drop_files,
            auto_close: self.auto_close,
        }
    }

    #[inline]
    pub fn inner_size<Coord>(
        self,
        size: Size<u32, Coord>,
    ) -> WindowBuilder<'a, Rx, Title, Size<u32, Coord>, Sty> {
        WindowBuilder {
            event_rx: self.event_rx,
            title: self.title,
            inner_size: size,
            style: self.style,
            visibility: self.visibility,
            enable_ime: self.enable_ime,
            visible_ime_candidate_window: self.visible_ime_candidate_window,
            accept_drop_files: self.accept_drop_files,
            auto_close: self.auto_close,
        }
    }

    #[inline]
    pub fn style<T>(self, style: T) -> WindowBuilder<'a, Rx, Title, Sz, T>
    where
        T: Style,
    {
        WindowBuilder {
            event_rx: self.event_rx,
            title: self.title,
            inner_size: self.inner_size,
            style,
            visibility: self.visibility,
            enable_ime: self.enable_ime,
            visible_ime_candidate_window: self.visible_ime_candidate_window,
            accept_drop_files: self.accept_drop_files,
            auto_close: self.auto_close,
        }
    }

    #[inline]
    pub fn visible(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    #[inline]
    pub fn enable_ime(mut self, enable: bool) -> Self {
        self.enable_ime = enable;
        self
    }

    #[inline]
    pub fn visible_ime_candidate_window(mut self, visiblity: bool) -> Self {
        self.visible_ime_candidate_window = visiblity;
        self
    }

    #[inline]
    pub fn accept_drop_files(mut self, accept: bool) -> Self {
        self.accept_drop_files = accept;
        self
    }

    #[inline]
    pub fn auto_close(mut self, flag: bool) -> Self {
        self.auto_close = flag;
        self
    }
}

struct BuilderProps<Sz> {
    title: HSTRING,
    inner_size: Sz,
    style: WINDOW_STYLE,
    ex_style: WINDOW_EX_STYLE,
    visiblity: bool,
    enable_ime: bool,
    visible_ime_candidate_window: bool,
    accept_drop_files: bool,
    auto_close: bool,
    event_rx_id: u64,
}

impl<Sz> BuilderProps<Sz> {
    fn new<Rx, Title, Sty>(builder: WindowBuilder<Rx, Title, Sz, Sty>, event_rx_id: u64) -> Self
    where
        Title: Into<String>,
        Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
        Sty: Style,
    {
        Self {
            title: HSTRING::from(builder.title.into()),
            inner_size: builder.inner_size,
            style: builder.style.style(),
            ex_style: builder.style.ex_style(),
            visiblity: builder.visibility,
            enable_ime: builder.enable_ime,
            visible_ime_candidate_window: builder.visible_ime_candidate_window,
            accept_drop_files: builder.accept_drop_files,
            auto_close: builder.auto_close,
            event_rx_id,
        }
    }
}

pub(crate) struct WindowProps {
    pub imm_context: ime::ImmContext,
    pub visible_ime_candidate_window: bool,
    pub auto_close: bool,
}

fn create_window<Sz>(props: BuilderProps<Sz>) -> Result<HWND>
where
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
{
    unsafe {
        let dpi = get_dpi_from_point(ScreenPosition::new(0, 0));
        let size = props.inner_size.to_physical(dpi);
        let rc = adjust_window_rect_ex_for_dpi(size, props.style, false, props.ex_style, dpi);
        let hinstance = GetModuleHandleW(None).unwrap();
        let hwnd = CreateWindowExW(
            props.ex_style,
            WINDOW_CLASS_NAME,
            &props.title,
            props.style,
            0,
            0,
            rc.right - rc.left,
            rc.bottom - rc.top,
            None,
            None,
            hinstance,
            None,
        );
        if hwnd == HWND(0) {
            return Err(Error::from_win32());
        }
        let imm_context = ime::ImmContext::new(hwnd);
        if props.enable_ime {
            imm_context.enable();
        } else {
            imm_context.disable();
        }
        DragAcceptFiles(hwnd, props.accept_drop_files);
        let window_props = WindowProps {
            imm_context,
            visible_ime_candidate_window: props.visible_ime_candidate_window,
            auto_close: props.auto_close,
        };
        Context::register_window(hwnd, window_props, props.event_rx_id);
        if props.visiblity {
            ShowWindow(hwnd, SW_SHOW);
        }
        Ok(hwnd)
    }
}

impl<'a, Title, Sz, Sty> WindowBuilder<'a, EventReceiver, Title, Sz, Sty>
where
    Title: Into<String>,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    Sty: Style,
{
    pub fn build(self) -> Result<Window> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<HWND>>();
        let event_rx_id = self.event_rx.id;
        let props = BuilderProps::new(self, event_rx_id);
        UiThread::send_task(move || {
            tx.send(create_window(props)).ok();
        });
        let Ok(ret) = rx.blocking_recv() else {
            return Err(Error::UiThreadClosed);
        };
        let hwnd = ret?;
        Ok(Window { hwnd })
    }
}

impl<'a, Title, Sz, Sty> WindowBuilder<'a, AsyncEventReceiver, Title, Sz, Sty>
where
    Title: Into<String>,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    Sty: Style,
{
    pub async fn build(self) -> Result<AsyncWindow> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<HWND>>();
        let event_rx_id = self.event_rx.id;
        let props = BuilderProps::new(self, event_rx_id);
        UiThread::send_task(move || {
            tx.send(create_window(props)).ok();
        });
        let Ok(ret) = rx.await else {
            return Err(Error::UiThreadClosed);
        };
        let hwnd = ret?;
        Ok(AsyncWindow { hwnd })
    }
}

impl<'a, Title, Sz> std::future::IntoFuture for WindowBuilder<'a, AsyncEventReceiver, Title, Sz>
where
    Title: Into<String>,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
{
    type Output = Result<AsyncWindow>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<HWND>>();
        let event_rx_id = self.event_rx.id;
        let props = BuilderProps::new(self, event_rx_id);
        UiThread::send_task(move || {
            tx.send(create_window(props)).ok();
        });
        Box::pin(async move {
            let Ok(ret) = rx.await else {
                return Err(Error::UiThreadClosed);
            };
            let hwnd = ret?;
            Ok(AsyncWindow { hwnd })
        })
    }
}

mod methods {
    use super::*;

    #[inline]
    pub fn position(hwnd: HWND) -> oneshot::Receiver<PhysicalPosition<i32>> {
        let (tx, rx) = oneshot::channel::<PhysicalPosition<i32>>();
        UiThread::send_task(move || {
            let rc = get_window_rect(hwnd);
            tx.send(PhysicalPosition::new(rc.left, rc.top)).ok();
        });
        rx
    }

    #[inline]
    pub fn inner_size(hwnd: HWND) -> oneshot::Receiver<PhysicalSize<u32>> {
        let (tx, rx) = oneshot::channel::<PhysicalSize<u32>>();
        UiThread::send_task(move || {
            let rc = get_client_rect(hwnd);
            tx.send(PhysicalSize::new(
                (rc.right - rc.left) as u32,
                (rc.bottom - rc.top) as u32,
            ))
            .ok();
        });
        rx
    }

    #[inline]
    pub fn dpi(hwnd: HWND) -> oneshot::Receiver<u32> {
        let (tx, rx) = oneshot::channel::<u32>();
        UiThread::send_task(move || unsafe {
            let dpi = GetDpiForWindow(hwnd);
            tx.send(dpi).ok();
        });
        rx
    }

    #[inline]
    pub fn enable_ime(hwnd: HWND, enabled: bool) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel::<()>();
        UiThread::send_task(move || {
            Context::set_window_props(hwnd, |props| {
                if enabled {
                    props.imm_context.enable();
                } else {
                    props.imm_context.disable();
                }
            });
            tx.send(()).ok();
        });
        rx
    }

    #[inline]
    pub fn set_position<T>(hwnd: HWND, position: T)
    where
        T: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    {
        UiThread::send_task(move || unsafe {
            let dpi = GetDpiForWindow(hwnd) as i32;
            let position = position.to_physical(dpi);
            SetWindowPos(
                hwnd,
                None,
                position.x,
                position.y,
                0,
                0,
                SWP_NOZORDER | SWP_NOSIZE,
            )
            .ok();
        });
    }

    #[inline]
    pub fn set_size<T>(hwnd: HWND, size: T)
    where
        T: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        UiThread::send_task(move || unsafe {
            let dpi = GetDpiForWindow(hwnd);
            let size = size.to_physical(dpi);
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                size.width as i32,
                size.height as i32,
                SWP_NOZORDER | SWP_NOSIZE,
            )
            .ok();
        });
    }

    #[inline]
    pub fn close(hwnd: HWND) {
        UiThread::send_task(move || unsafe {
            PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0)).ok();
        });
    }
}

pub struct Window {
    hwnd: HWND,
}

impl Window {
    pub(crate) fn from_isize(hwnd: isize) -> Self {
        Self { hwnd: HWND(hwnd) }
    }

    #[inline]
    pub fn builder<T>(event_rx: &T) -> WindowBuilder<T> {
        WindowBuilder::new(event_rx)
    }

    #[inline]
    pub fn position(&self) -> Option<PhysicalPosition<i32>> {
        let rx = methods::position(self.hwnd);
        rx.blocking_recv().ok()
    }

    #[inline]
    pub fn inner_size(&self) -> Option<PhysicalSize<u32>> {
        let rx = methods::inner_size(self.hwnd);
        rx.blocking_recv().ok()
    }

    #[inline]
    pub fn dpi(&self) -> Option<u32> {
        let rx = methods::dpi(self.hwnd);
        rx.blocking_recv().ok()
    }

    #[inline]
    pub fn set_position<T>(&self, position: T)
    where
        T: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    {
        methods::set_position(self.hwnd, position);
    }

    #[inline]
    pub fn set_size<T>(&self, size: T)
    where
        T: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        methods::set_size(self.hwnd, size);
    }

    #[inline]
    pub fn enable_ime(&self, enabled: bool) {
        methods::enable_ime(self.hwnd, enabled).blocking_recv().ok();
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        Context::window_is_none(self.hwnd)
    }

    #[inline]
    pub fn close(&self) {
        methods::close(self.hwnd);
    }
}

pub struct AsyncWindow {
    hwnd: HWND,
}

impl AsyncWindow {
    #[inline]
    pub fn builder(event_rx: &AsyncEventReceiver) -> WindowBuilder<AsyncEventReceiver> {
        WindowBuilder::new(event_rx)
    }

    #[inline]
    pub async fn position(&self) -> Option<PhysicalPosition<i32>> {
        let rx = methods::position(self.hwnd);
        rx.await.ok()
    }

    #[inline]
    pub async fn inner_size(&self) -> Option<PhysicalSize<u32>> {
        let rx = methods::inner_size(self.hwnd);
        rx.await.ok()
    }

    #[inline]
    pub async fn dpi(&self) -> Option<u32> {
        let rx = methods::dpi(self.hwnd);
        rx.await.ok()
    }

    #[inline]
    pub fn set_position<T>(&self, position: T)
    where
        T: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    {
        methods::set_position(self.hwnd, position);
    }

    #[inline]
    pub fn set_size<T>(&self, size: T)
    where
        T: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        methods::set_size(self.hwnd, size);
    }

    #[inline]
    pub async fn enable_ime(&self, enabled: bool) {
        methods::enable_ime(self.hwnd, enabled).await.ok();
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        Context::window_is_none(self.hwnd)
    }

    #[inline]
    pub fn close(&self) {
        methods::close(self.hwnd);
    }
}

impl raw_window_handle::HasWindowHandle for Window {
    #[inline]
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        Ok(unsafe {
            raw_window_handle::WindowHandle::borrow_raw(raw_window_handle::RawWindowHandle::Win32(
                raw_window_handle::Win32WindowHandle::new(self.hwnd.0.try_into().unwrap()),
            ))
        })
    }
}

impl raw_window_handle::HasWindowHandle for AsyncWindow {
    #[inline]
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        Ok(unsafe {
            raw_window_handle::WindowHandle::borrow_raw(raw_window_handle::RawWindowHandle::Win32(
                raw_window_handle::Win32WindowHandle::new(self.hwnd.0.try_into().unwrap()),
            ))
        })
    }
}

impl raw_window_handle::HasDisplayHandle for Window {
    #[inline]
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        Ok(unsafe {
            raw_window_handle::DisplayHandle::borrow_raw(
                raw_window_handle::RawDisplayHandle::Windows(
                    raw_window_handle::WindowsDisplayHandle::new(),
                ),
            )
        })
    }
}

impl raw_window_handle::HasDisplayHandle for AsyncWindow {
    #[inline]
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        Ok(unsafe {
            raw_window_handle::DisplayHandle::borrow_raw(
                raw_window_handle::RawDisplayHandle::Windows(
                    raw_window_handle::WindowsDisplayHandle::new(),
                ),
            )
        })
    }
}
