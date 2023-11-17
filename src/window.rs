use crate::*;
use std::any::Any;
use std::cell::RefCell;
use std::sync::atomic::{self, AtomicU64};
use tokio::sync::oneshot;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::{
    Foundation::{BOOL, HINSTANCE, HWND, LPARAM, WPARAM},
    Graphics::Dwm::*,
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
            lpszClassName: WINDOW_CLASS_NAME,
            hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 {
            panic!("RegisterClassExW failed");
        }
    }
}

/// `EventReceive` and `AsyncEventReceiver` are implement this trait.
pub trait IsReceiver {
    fn id(&self) -> u64;
}

/// `Window` and `AsyncWindow` are implement this trait.
pub trait IsWindow {
    fn hwnd(&self) -> HWND;
}

/// Represents `Window` or `InnerWindow`
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WindowKind {
    Window(Window),
    InnerWindow(InnerWindow),
}

impl WindowKind {
    pub(crate) fn hwnd(&self) -> HWND {
        match self {
            Self::Window(w) => w.hwnd,
            Self::InnerWindow(w) => w.hwnd,
        }
    }
}

/// Represents `AsyncWindow` or `AsyncInnerWindow`
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AsyncWindowKind {
    Window(AsyncWindow),
    InnerWindow(AsyncInnerWindow),
}

impl AsyncWindowKind {
    pub(crate) fn hwnd(&self) -> HWND {
        match self {
            Self::Window(w) => w.hwnd,
            Self::InnerWindow(w) => w.hwnd,
        }
    }
}

type Receiver<T> = tokio::sync::mpsc::UnboundedReceiver<T>;
pub(crate) type Sender<T> = tokio::sync::mpsc::UnboundedSender<T>;

pub(crate) enum RecvEventOrPanic {
    Event(RecvEvent),
    Panic(Box<dyn Any + Send>),
}

/// The type of receiving event from UI thread.
pub type RecvEvent = (Event, WindowKind);

/// The async version type of receiving event from UI thread.
pub type AsyncRecvEvent = (Event, AsyncWindowKind);

fn gen_id() -> u64 {
    static ID: AtomicU64 = AtomicU64::new(0);
    ID.fetch_add(1, atomic::Ordering::SeqCst)
}

/// This object which receives an event from UI thread.
pub struct EventReceiver {
    id: u64,
    rx: RefCell<Receiver<RecvEventOrPanic>>,
}

impl EventReceiver {
    /// Creates a new event receiver.
    #[inline]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let id = gen_id();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Context::register_event_tx(id, tx);
        Self {
            id,
            rx: RefCell::new(rx),
        }
    }

    /// Attempts to wait for an event from UI thread.
    ///
    /// When UI thread shutdowns, this function returns `None`.
    ///
    #[inline]
    pub fn recv(&self) -> Option<RecvEvent> {
        match self.rx.borrow_mut().blocking_recv()? {
            RecvEventOrPanic::Event(ret) => Some(ret),
            RecvEventOrPanic::Panic(e) => std::panic::resume_unwind(e),
        }
    }

    /// Attempts to return a pending event from UI Thread.
    ///
    /// This function do not block.
    /// When UI thread shutdowns, this function returns `Err(TryRecvError::Disconnected)`.
    ///
    #[inline]
    pub fn try_recv(&self) -> TryRecvResult<RecvEvent> {
        use tokio::sync::mpsc;

        match self.rx.borrow_mut().try_recv() {
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

/// This object which receives an event from UI thread.
pub struct AsyncEventReceiver {
    id: u64,
    rx: RefCell<Receiver<RecvEventOrPanic>>,
}

impl AsyncEventReceiver {
    /// Creates a new event receiver.
    #[inline]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let id = gen_id();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Context::register_event_tx(id, tx);
        Self {
            id,
            rx: RefCell::new(rx),
        }
    }

    /// Attempts to wait for an event from UI thread.
    ///
    /// When UI thread shutdowns, this function returns `None`.
    ///
    #[inline]
    pub async fn recv(&self) -> Option<AsyncRecvEvent> {
        let ret = match self.rx.borrow_mut().recv().await? {
            RecvEventOrPanic::Event(re) => re,
            RecvEventOrPanic::Panic(e) => std::panic::resume_unwind(e),
        };
        match ret.1 {
            WindowKind::Window(w) => {
                Some((ret.0, AsyncWindowKind::Window(AsyncWindow { hwnd: w.hwnd })))
            }
            WindowKind::InnerWindow(w) => Some((
                ret.0,
                AsyncWindowKind::InnerWindow(AsyncInnerWindow { hwnd: w.hwnd }),
            )),
        }
    }

    /// Attempts to return a pending event from UI Thread.
    ///
    /// This function do not block.
    /// When UI thread shutdowns, this function returns `Err(TryRecvError::Disconnected)`.
    ///
    #[inline]
    pub fn try_recv(&self) -> TryRecvResult<AsyncRecvEvent> {
        use tokio::sync::mpsc;

        match self.rx.borrow_mut().try_recv() {
            Ok(ret) => {
                let ret = match ret {
                    RecvEventOrPanic::Event(re) => re,
                    RecvEventOrPanic::Panic(e) => std::panic::resume_unwind(e),
                };
                match ret.1 {
                    WindowKind::Window(w) => {
                        Ok((ret.0, AsyncWindowKind::Window(AsyncWindow { hwnd: w.hwnd })))
                    }
                    WindowKind::InnerWindow(w) => Ok((
                        ret.0,
                        AsyncWindowKind::InnerWindow(AsyncInnerWindow { hwnd: w.hwnd }),
                    )),
                }
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

/// Builds a window.
pub struct WindowBuilder<'a, Rx, Title = &'static str, Sz = LogicalSize<u32>, Sty = WindowStyle> {
    event_rx: &'a Rx,
    title: Title,
    position: PhysicalPosition<i32>,
    inner_size: Sz,
    style: Sty,
    visibility: bool,
    enable_ime: bool,
    visible_ime_candidate_window: bool,
    accept_drop_files: bool,
    auto_close: bool,
    nc_hittest: bool,
    icon: Option<Icon>,
    cursor: Cursor,
    parent: Option<HWND>,
}

impl<'a, Rx> WindowBuilder<'a, Rx> {
    /// Creates a window builder.
    #[inline]
    pub fn new(event_rx: &'a Rx) -> Self {
        UiThread::init();
        Self {
            event_rx,
            title: "",
            position: PhysicalPosition::new(CW_USEDEFAULT, CW_USEDEFAULT),
            inner_size: LogicalSize::new(1024, 768),
            style: WindowStyle::default(),
            visibility: true,
            enable_ime: true,
            visible_ime_candidate_window: true,
            accept_drop_files: false,
            auto_close: true,
            nc_hittest: false,
            icon: None,
            cursor: Cursor::default(),
            parent: None,
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
            position: self.position,
            inner_size: self.inner_size,
            style: self.style,
            visibility: self.visibility,
            enable_ime: self.enable_ime,
            visible_ime_candidate_window: self.visible_ime_candidate_window,
            accept_drop_files: self.accept_drop_files,
            auto_close: self.auto_close,
            nc_hittest: self.nc_hittest,
            icon: self.icon,
            cursor: self.cursor,
            parent: self.parent,
        }
    }

    #[inline]
    pub fn position(mut self, position: ScreenPosition<i32>) -> Self {
        self.position = PhysicalPosition::new(position.x, position.y);
        self
    }

    #[inline]
    pub fn inner_size<Coord>(
        self,
        size: Size<u32, Coord>,
    ) -> WindowBuilder<'a, Rx, Title, Size<u32, Coord>, Sty> {
        WindowBuilder {
            event_rx: self.event_rx,
            title: self.title,
            position: self.position,
            inner_size: size,
            style: self.style,
            visibility: self.visibility,
            enable_ime: self.enable_ime,
            visible_ime_candidate_window: self.visible_ime_candidate_window,
            accept_drop_files: self.accept_drop_files,
            auto_close: self.auto_close,
            nc_hittest: self.nc_hittest,
            icon: self.icon,
            cursor: self.cursor,
            parent: self.parent,
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
            position: self.position,
            inner_size: self.inner_size,
            style,
            visibility: self.visibility,
            enable_ime: self.enable_ime,
            visible_ime_candidate_window: self.visible_ime_candidate_window,
            accept_drop_files: self.accept_drop_files,
            auto_close: self.auto_close,
            nc_hittest: self.nc_hittest,
            icon: self.icon,
            cursor: self.cursor,
            parent: self.parent,
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

    #[inline]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    #[inline]
    pub fn cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = cursor;
        self
    }

    #[inline]
    pub fn parent(mut self, parent: &impl IsWindow) -> Self {
        self.parent = Some(parent.hwnd());
        self
    }

    #[inline]
    pub fn hook_nc_hittest(mut self, flag: bool) -> Self {
        self.nc_hittest = flag;
        self
    }
}

struct BuilderProps<Pos, Sz> {
    title: HSTRING,
    position: Pos,
    inner_size: Sz,
    style: WINDOW_STYLE,
    ex_style: WINDOW_EX_STYLE,
    visiblity: bool,
    enable_ime: bool,
    visible_ime_candidate_window: bool,
    accept_drop_files: bool,
    auto_close: bool,
    icon: Option<Icon>,
    cursor: Cursor,
    nc_hittest: bool,
    event_rx_id: u64,
    parent: Option<HWND>,
    parent_inner: Option<HWND>,
}

impl<Sz> BuilderProps<PhysicalPosition<i32>, Sz> {
    fn new<Rx, Title, Sty>(builder: WindowBuilder<Rx, Title, Sz, Sty>) -> Self
    where
        Rx: IsReceiver,
        Title: Into<String>,
        Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
        Sty: Style,
    {
        Self {
            title: HSTRING::from(builder.title.into()),
            position: builder.position,
            inner_size: builder.inner_size,
            style: builder.style.style(),
            ex_style: builder.style.ex_style(),
            visiblity: builder.visibility,
            enable_ime: builder.enable_ime,
            visible_ime_candidate_window: builder.visible_ime_candidate_window,
            accept_drop_files: builder.accept_drop_files,
            auto_close: builder.auto_close,
            icon: builder.icon,
            cursor: builder.cursor,
            nc_hittest: builder.nc_hittest,
            event_rx_id: builder.event_rx.id(),
            parent: builder.parent,
            parent_inner: None,
        }
    }
}

impl<Pos, Sz> BuilderProps<Pos, Sz> {
    fn new_inner<Rx>(builder: InnerWindowBuilder<Rx, Pos, Sz>) -> Self
    where
        Rx: IsReceiver,
        Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        Self {
            title: HSTRING::from(String::new()),
            position: builder.position,
            inner_size: builder.size,
            style: WS_CHILD,
            ex_style: WINDOW_EX_STYLE(0),
            visiblity: builder.visibility,
            enable_ime: builder.enable_ime,
            visible_ime_candidate_window: builder.visible_ime_candidate_window,
            accept_drop_files: builder.accept_drop_files,
            auto_close: true,
            icon: None,
            cursor: builder.cursor,
            nc_hittest: builder.nc_hittest,
            event_rx_id: builder.event_rx.id(),
            parent: None,
            parent_inner: Some(builder.parent_inner),
        }
    }
}

pub(crate) struct WindowProps {
    pub imm_context: ime::ImmContext,
    pub visible_ime_candidate_window: bool,
    pub auto_close: bool,
    pub cursor: Cursor,
    pub parent: Option<HWND>,
    pub nc_hittest: bool,
}

fn create_window<Pos, Sz>(
    props: BuilderProps<Pos, Sz>,
    f: impl FnOnce(HWND) -> WindowKind,
) -> Result<HWND>
where
    Pos: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
{
    unsafe {
        let dpi = get_dpi_from_point(ScreenPosition::new(0, 0));
        let position = props.position.to_physical(dpi as i32);
        let size = props.inner_size.to_physical(dpi);
        let rc = adjust_window_rect_ex_for_dpi(size, props.style, false, props.ex_style, dpi);
        let hinstance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        let hwnd = CreateWindowExW(
            props.ex_style,
            WINDOW_CLASS_NAME,
            &props.title,
            props.style,
            position.x,
            position.y,
            rc.right - rc.left,
            rc.bottom - rc.top,
            props.parent_inner.as_ref(),
            None,
            hinstance,
            None,
        );
        if hwnd == HWND(0) {
            return Err(Error::from_win32());
        }
        let dark_mode = BOOL::from(is_dark_mode());
        if dark_mode.as_bool() {
            log::info!("Dark mode");
        } else {
            log::info!("Light mode");
        }
        let ret = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_mode as *const BOOL as *const std::ffi::c_void,
            std::mem::size_of::<BOOL>() as u32,
        );
        if let Err(e) = ret {
            log::error!("DwmSetWindowAttribute: {e}");
        }
        let imm_context = ime::ImmContext::new(hwnd);
        if props.enable_ime {
            imm_context.enable();
        } else {
            imm_context.disable();
        }
        if let Some(icon) = props.icon {
            if let Ok(big) = icon.load(hinstance) {
                PostMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as usize), LPARAM(big.0)).ok();
            }
            if let Ok(small) = icon.load_small(hinstance) {
                PostMessageW(
                    hwnd,
                    WM_SETICON,
                    WPARAM(ICON_SMALL as usize),
                    LPARAM(small.0),
                )
                .ok();
            }
        }
        DragAcceptFiles(hwnd, props.accept_drop_files);
        let window_props = WindowProps {
            imm_context,
            visible_ime_candidate_window: props.visible_ime_candidate_window,
            auto_close: props.auto_close,
            cursor: props.cursor,
            parent: props.parent,
            nc_hittest: props.nc_hittest,
        };
        Context::register_window(f(hwnd), window_props, props.event_rx_id);
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
    /// Builds a window.
    pub fn build(self) -> Result<Window> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<HWND>>();
        let props = BuilderProps::new(self);
        UiThread::send_task(move || {
            tx.send(create_window(props, |hwnd| {
                WindowKind::Window(Window { hwnd })
            }))
            .ok();
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
    /// Builds a window.
    pub async fn build(self) -> Result<AsyncWindow> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<HWND>>();
        let props = BuilderProps::new(self);
        UiThread::send_task(move || {
            tx.send(create_window(props, |hwnd| {
                WindowKind::Window(Window { hwnd })
            }))
            .ok();
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
        let props = BuilderProps::new(self);
        UiThread::send_task(move || {
            tx.send(create_window(props, |hwnd| {
                WindowKind::InnerWindow(InnerWindow { hwnd })
            }))
            .ok();
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

    #[inline]
    pub fn show(hwnd: HWND) {
        unsafe {
            ShowWindowAsync(hwnd, SW_SHOW);
        }
    }

    #[inline]
    pub fn hide(hwnd: HWND) {
        unsafe {
            ShowWindowAsync(hwnd, SW_HIDE);
        }
    }

    #[inline]
    pub fn minimize(hwnd: HWND) {
        unsafe {
            ShowWindowAsync(hwnd, SW_MINIMIZE);
        }
    }

    #[inline]
    pub fn maximize(hwnd: HWND) {
        unsafe {
            ShowWindowAsync(hwnd, SW_SHOWMAXIMIZED);
        }
    }

    #[inline]
    pub fn restore(hwnd: HWND) {
        unsafe {
            ShowWindowAsync(hwnd, SW_RESTORE);
        }
    }

    #[inline]
    pub fn set_cursor(hwnd: HWND, cursor: Cursor) {
        UiThread::send_task(move || {
            cursor.set();
            Context::set_window_props(hwnd, |props| {
                props.cursor = cursor;
            });
        });
    }
}

/// Represents a window.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Window {
    hwnd: HWND,
}

impl Window {
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
    pub fn set_inner_size<T>(&self, size: T)
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
    pub fn show(&self) {
        methods::show(self.hwnd);
    }

    #[inline]
    pub fn hide(&self) {
        methods::hide(self.hwnd);
    }

    #[inline]
    pub fn minimize(&self) {
        methods::minimize(self.hwnd);
    }

    #[inline]
    pub fn maximize(&self) {
        methods::maximize(self.hwnd);
    }

    #[inline]
    pub fn restore(&self) {
        methods::restore(self.hwnd);
    }

    #[inline]
    pub fn cursor(&self) -> Option<Cursor> {
        Context::get_window_props(self.hwnd, |props| props.cursor.clone())
    }

    #[inline]
    pub fn set_cursor(&self, cursor: Cursor) {
        let hwnd = self.hwnd;
        methods::set_cursor(hwnd, cursor);
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        Context::window_is_none(self.hwnd)
    }

    #[inline]
    pub fn close(&self) {
        methods::close(self.hwnd);
    }

    #[inline]
    pub fn raw_handle(&self) -> isize {
        self.hwnd.0
    }
}

impl IsWindow for Window {
    #[inline]
    fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

/// Represents a window of async version.
#[derive(Clone, PartialEq, Eq, Debug)]
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
    pub fn set_inner_size<T>(&self, size: T)
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
    pub fn show(&self) {
        methods::show(self.hwnd);
    }

    #[inline]
    pub fn hide(&self) {
        methods::hide(self.hwnd);
    }

    #[inline]
    pub fn minimize(&self) {
        methods::minimize(self.hwnd);
    }

    #[inline]
    pub fn maximize(&self) {
        methods::maximize(self.hwnd);
    }

    #[inline]
    pub fn restore(&self) {
        methods::restore(self.hwnd);
    }

    #[inline]
    pub fn cursor(&self) -> Option<Cursor> {
        Context::get_window_props(self.hwnd, |props| props.cursor.clone())
    }

    #[inline]
    pub fn set_cursor(&self, cursor: Cursor) {
        let hwnd = self.hwnd;
        methods::set_cursor(hwnd, cursor);
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        Context::window_is_none(self.hwnd)
    }

    #[inline]
    pub fn close(&self) {
        methods::close(self.hwnd);
    }

    #[inline]
    pub fn raw_handle(&self) -> isize {
        self.hwnd.0
    }
}

impl IsWindow for AsyncWindow {
    #[inline]
    fn hwnd(&self) -> HWND {
        self.hwnd
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

/// Builds a inner window.
///
/// Needs specifying position and size of an inner window.
///
pub struct InnerWindowBuilder<'a, Rx, Pos = (), Sz = ()> {
    event_rx: &'a Rx,
    parent_inner: HWND,
    position: Pos,
    size: Sz,
    visibility: bool,
    enable_ime: bool,
    visible_ime_candidate_window: bool,
    accept_drop_files: bool,
    cursor: Cursor,
    nc_hittest: bool,
}

impl<'a, Rx> InnerWindowBuilder<'a, Rx> {
    /// Creates an inner window builder.
    #[inline]
    pub fn new<T>(event_rx: &'a Rx, parent: &T) -> Self
    where
        T: IsWindow,
    {
        Self {
            event_rx,
            parent_inner: parent.hwnd(),
            position: (),
            size: (),
            visibility: true,
            enable_ime: true,
            visible_ime_candidate_window: true,
            accept_drop_files: false,
            cursor: Context::get_window_props(parent.hwnd(), |props| props.cursor.clone())
                .unwrap_or_default(),
            nc_hittest: false,
        }
    }
}

impl<'a, Rx, Pos, Sz> InnerWindowBuilder<'a, Rx, Pos, Sz> {
    #[inline]
    pub fn position<T>(self, position: T) -> InnerWindowBuilder<'a, Rx, T, Sz>
    where
        T: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    {
        InnerWindowBuilder {
            event_rx: self.event_rx,
            parent_inner: self.parent_inner,
            position,
            size: self.size,
            visibility: self.visibility,
            enable_ime: self.enable_ime,
            visible_ime_candidate_window: self.visible_ime_candidate_window,
            accept_drop_files: self.accept_drop_files,
            cursor: self.cursor,
            nc_hittest: self.nc_hittest,
        }
    }

    #[inline]
    pub fn size<T>(self, size: T) -> InnerWindowBuilder<'a, Rx, Pos, T>
    where
        T: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        InnerWindowBuilder {
            event_rx: self.event_rx,
            parent_inner: self.parent_inner,
            position: self.position,
            size,
            visibility: self.visibility,
            enable_ime: self.enable_ime,
            visible_ime_candidate_window: self.visible_ime_candidate_window,
            accept_drop_files: self.accept_drop_files,
            cursor: self.cursor,
            nc_hittest: self.nc_hittest,
        }
    }

    #[inline]
    pub fn visible(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    #[inline]
    pub fn enable_ime(mut self, flag: bool) -> Self {
        self.enable_ime = flag;
        self
    }

    #[inline]
    pub fn visible_ime_candidate_window(mut self, flag: bool) -> Self {
        self.visible_ime_candidate_window = flag;
        self
    }

    #[inline]
    pub fn accept_drop_files(mut self, flag: bool) -> Self {
        self.accept_drop_files = flag;
        self
    }

    #[inline]
    pub fn hook_nc_hittest(mut self, flag: bool) -> Self {
        self.nc_hittest = flag;
        self
    }
}

impl<'a, Pos, Sz> InnerWindowBuilder<'a, EventReceiver, Pos, Sz>
where
    Pos: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
{
    /// Builds an inner window.
    #[inline]
    pub fn build(self) -> Result<InnerWindow> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<HWND>>();
        let props = BuilderProps::new_inner(self);
        UiThread::send_task(move || {
            tx.send(create_window(props, |hwnd| {
                WindowKind::Window(Window { hwnd })
            }))
            .ok();
        });
        let Ok(ret) = rx.blocking_recv() else {
            return Err(Error::UiThreadClosed);
        };
        let hwnd = ret?;
        Ok(InnerWindow { hwnd })
    }
}

impl<'a, Pos, Sz> InnerWindowBuilder<'a, AsyncEventReceiver, Pos, Sz>
where
    Pos: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
{
    /// Builds an inner window of async version.
    #[inline]
    pub async fn build(self) -> Result<AsyncInnerWindow> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<HWND>>();
        let props = BuilderProps::new_inner(self);
        UiThread::send_task(move || {
            tx.send(create_window(props, |hwnd| {
                WindowKind::InnerWindow(InnerWindow { hwnd })
            }))
            .ok();
        });
        let Ok(ret) = rx.await else {
            return Err(Error::UiThreadClosed);
        };
        let hwnd = ret?;
        Ok(AsyncInnerWindow { hwnd })
    }
}

/// Represents an inner window.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InnerWindow {
    hwnd: HWND,
}

impl InnerWindow {
    #[inline]
    pub fn builder<'a, Rx, T>(event_tx: &'a Rx, parent: &T) -> InnerWindowBuilder<'a, Rx>
    where
        Rx: IsReceiver,
        T: IsWindow,
    {
        InnerWindowBuilder::new(event_tx, parent)
    }

    #[inline]
    pub fn position(&self) -> Option<PhysicalPosition<i32>> {
        let rx = methods::position(self.hwnd);
        rx.blocking_recv().ok()
    }

    #[inline]
    pub fn size(&self) -> Option<PhysicalSize<u32>> {
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
    pub fn cursor(&self) -> Option<Cursor> {
        Context::get_window_props(self.hwnd, |props| props.cursor.clone())
    }

    #[inline]
    pub fn set_cursor(&self, cursor: Cursor) {
        let hwnd = self.hwnd;
        methods::set_cursor(hwnd, cursor);
    }

    #[inline]
    pub fn raw_handle(&self) -> isize {
        self.hwnd.0
    }
}

/// Represents an inner window of async version.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AsyncInnerWindow {
    hwnd: HWND,
}

impl AsyncInnerWindow {
    #[inline]
    pub fn builder<'a, T>(
        event_tx: &'a AsyncEventReceiver,
        parent: &AsyncWindow,
    ) -> InnerWindowBuilder<'a, AsyncEventReceiver>
    where
        T: IsWindow,
    {
        InnerWindowBuilder::new(event_tx, parent)
    }

    #[inline]
    pub async fn position(&self) -> Option<PhysicalPosition<i32>> {
        let rx = methods::position(self.hwnd);
        rx.await.ok()
    }

    #[inline]
    pub async fn size(&self) -> Option<PhysicalSize<u32>> {
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
    pub fn enable_ime(&self, enabled: bool) {
        methods::enable_ime(self.hwnd, enabled).blocking_recv().ok();
    }

    #[inline]
    pub fn cursor(&self) -> Option<Cursor> {
        Context::get_window_props(self.hwnd, |props| props.cursor.clone())
    }

    #[inline]
    pub fn set_cursor(&self, cursor: Cursor) {
        let hwnd = self.hwnd;
        methods::set_cursor(hwnd, cursor);
    }

    #[inline]
    pub fn raw_handle(&self) -> isize {
        self.hwnd.0
    }
}

impl raw_window_handle::HasWindowHandle for InnerWindow {
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

impl raw_window_handle::HasWindowHandle for AsyncInnerWindow {
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

impl raw_window_handle::HasDisplayHandle for InnerWindow {
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

impl raw_window_handle::HasDisplayHandle for AsyncInnerWindow {
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

impl PartialEq<Window> for WindowKind {
    #[inline]
    fn eq(&self, other: &Window) -> bool {
        self.hwnd() == other.hwnd
    }
}

impl PartialEq<InnerWindow> for WindowKind {
    #[inline]
    fn eq(&self, other: &InnerWindow) -> bool {
        self.hwnd() == other.hwnd
    }
}

impl PartialEq<WindowKind> for Window {
    #[inline]
    fn eq(&self, other: &WindowKind) -> bool {
        self.hwnd == other.hwnd()
    }
}

impl PartialEq<WindowKind> for InnerWindow {
    #[inline]
    fn eq(&self, other: &WindowKind) -> bool {
        self.hwnd == other.hwnd()
    }
}

impl PartialEq<AsyncWindowKind> for AsyncWindow {
    #[inline]
    fn eq(&self, other: &AsyncWindowKind) -> bool {
        self.hwnd == other.hwnd()
    }
}

impl PartialEq<AsyncWindowKind> for AsyncInnerWindow {
    #[inline]
    fn eq(&self, other: &AsyncWindowKind) -> bool {
        self.hwnd == other.hwnd()
    }
}

impl PartialEq<AsyncWindow> for AsyncWindowKind {
    #[inline]
    fn eq(&self, other: &AsyncWindow) -> bool {
        self.hwnd() == other.hwnd
    }
}

impl PartialEq<AsyncInnerWindow> for AsyncWindowKind {
    #[inline]
    fn eq(&self, other: &AsyncInnerWindow) -> bool {
        self.hwnd() == other.hwnd
    }
}
