use crate::*;
use std::any::Any;
use std::sync::atomic::{self, AtomicU64};
use tokio::sync::oneshot;
use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, POINT, RECT, WPARAM},
    Graphics::Dwm::*,
    Graphics::Gdi::{GetStockObject, HBRUSH, ScreenToClient, WHITE_BRUSH},
    System::LibraryLoader::GetModuleHandleW,
    UI::HiDpi::GetDpiForWindow,
    UI::Input::KeyboardAndMouse::SetFocus,
    UI::Shell::DragAcceptFiles,
    UI::WindowsAndMessaging::*,
};
use windows::core::{BOOL, HSTRING, PCWSTR};

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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct WindowHandle(HWND);

impl WindowHandle {
    pub(crate) fn new(hwnd: HWND) -> Self {
        Self(hwnd)
    }

    #[inline]
    pub fn as_hwnd(&self) -> HWND {
        self.0
    }
}

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}

impl From<WindowHandle> for HWND {
    fn from(value: WindowHandle) -> Self {
        value.as_hwnd()
    }
}

impl std::hash::Hash for WindowHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.0.0.addr());
    }
}

/// `EventReceive` and `AsyncEventReceiver` are implement this trait.
pub trait IsReceiver {
    fn id(&self) -> u64;
}

/// `Window` and `AsyncWindow` are implement this trait.
pub trait IsWindow {
    fn window_handle(&self) -> WindowHandle;
}

/// Represents `Window` or `InnerWindow`
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WindowKind {
    Window(Window),
    InnerWindow(InnerWindow),
}

impl IsWindow for WindowKind {
    fn window_handle(&self) -> WindowHandle {
        match self {
            Self::Window(w) => w.handle,
            Self::InnerWindow(w) => w.handle,
        }
    }
}

/// Represents `AsyncWindow` or `AsyncInnerWindow`
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AsyncWindowKind {
    Window(AsyncWindow),
    InnerWindow(AsyncInnerWindow),
}

impl IsWindow for AsyncWindowKind {
    fn window_handle(&self) -> WindowHandle {
        match self {
            Self::Window(w) => w.handle,
            Self::InnerWindow(w) => w.handle,
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
    rx: Receiver<RecvEventOrPanic>,
}

impl EventReceiver {
    /// Creates a new event receiver.
    #[inline]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let id = gen_id();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Context::register_event_tx(id, tx);
        Self { id, rx }
    }

    /// Attempts to wait for an event from UI thread.
    ///
    /// When UI thread shutdowns, this function returns `None`.
    ///
    #[inline]
    pub fn recv(&mut self) -> Option<RecvEvent> {
        match self.rx.blocking_recv()? {
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

/// This object which receives an event from UI thread.
pub struct AsyncEventReceiver {
    id: u64,
    rx: Receiver<RecvEventOrPanic>,
}

impl AsyncEventReceiver {
    /// Creates a new event receiver.
    #[inline]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let id = gen_id();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Context::register_event_tx(id, tx);
        Self { id, rx }
    }

    /// Attempts to wait for an event from UI thread.
    ///
    /// When UI thread shutdowns, this function returns `None`.
    ///
    #[inline]
    pub async fn recv(&mut self) -> Option<AsyncRecvEvent> {
        let ret = {
            match self.rx.recv().await? {
                RecvEventOrPanic::Event(re) => re,
                RecvEventOrPanic::Panic(e) => std::panic::resume_unwind(e),
            }
        };
        match ret.1 {
            WindowKind::Window(w) => Some((
                ret.0,
                AsyncWindowKind::Window(AsyncWindow { handle: w.handle }),
            )),
            WindowKind::InnerWindow(w) => Some((
                ret.0,
                AsyncWindowKind::InnerWindow(AsyncInnerWindow {
                    parent: w.parent,
                    handle: w.handle,
                }),
            )),
        }
    }

    /// Attempts to return a pending event from UI Thread.
    ///
    /// This function do not block.
    /// When UI thread shutdowns, this function returns `Err(TryRecvError::Disconnected)`.
    ///
    #[inline]
    pub fn try_recv(&mut self) -> TryRecvResult<AsyncRecvEvent> {
        use tokio::sync::mpsc;

        match self.rx.try_recv() {
            Ok(ret) => {
                let ret = match ret {
                    RecvEventOrPanic::Event(re) => re,
                    RecvEventOrPanic::Panic(e) => std::panic::resume_unwind(e),
                };
                match ret.1 {
                    WindowKind::Window(w) => Ok((
                        ret.0,
                        AsyncWindowKind::Window(AsyncWindow { handle: w.handle }),
                    )),
                    WindowKind::InnerWindow(w) => Ok((
                        ret.0,
                        AsyncWindowKind::InnerWindow(AsyncInnerWindow {
                            parent: w.parent,
                            handle: w.handle,
                        }),
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
    parent: Option<WindowHandle>,
    menu: Option<MenuBar>,
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
            menu: None,
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
            menu: self.menu,
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
            menu: self.menu,
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
            menu: self.menu,
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
        self.parent = Some(parent.window_handle());
        self
    }

    #[inline]
    pub fn hook_nc_hittest(mut self, flag: bool) -> Self {
        self.nc_hittest = flag;
        self
    }

    #[inline]
    pub fn menu(mut self, menu: &MenuBar) -> Self {
        self.menu = Some(menu.clone());
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
    parent: Option<WindowHandle>,
    parent_inner: Option<WindowHandle>,
    menu: Option<MenuBar>,
    set_attr: bool,
    color_mode: ColorMode,
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
            menu: builder.menu,
            set_attr: true,
            color_mode: ColorMode::System,
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
            menu: None,
            set_attr: false,
            color_mode: ColorMode::System,
        }
    }
}

pub(crate) struct WindowProps {
    pub imm_context: ime::ImmContext,
    pub visible_ime_candidate_window: bool,
    pub auto_close: bool,
    pub cursor: Cursor,
    pub parent: Option<WindowHandle>,
    pub nc_hittest: bool,
    pub redrawing: bool,
    pub resizing: bool,
    pub minimized: bool,
    pub _menu: Option<MenuBar>,
    pub theme_menu: Theme,
    pub color_mode: ColorMode,
    pub color_mode_state: ColorModeState,
}

fn create_window<Pos, Sz>(
    props: BuilderProps<Pos, Sz>,
    f: impl FnOnce(WindowHandle) -> WindowKind,
) -> Result<WindowHandle>
where
    Pos: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
{
    unsafe {
        let dpi = get_dpi_from_point(ScreenPosition::new(0, 0));
        let position = props.position.to_physical(dpi as i32);
        let size = props.inner_size.to_physical(dpi);
        let rc = adjust_window_rect_ex_for_dpi(
            size,
            props.style,
            props.menu.is_some(),
            props.ex_style,
            dpi,
        );
        let hinstance: Option<HINSTANCE> = Some(GetModuleHandleW(None).unwrap().into());
        let parent = props.parent_inner.as_ref().map(|p| p.as_hwnd());
        let hwnd = CreateWindowExW(
            props.ex_style,
            WINDOW_CLASS_NAME,
            &props.title,
            props.style,
            position.x,
            position.y,
            rc.right - rc.left,
            rc.bottom - rc.top,
            parent,
            props.menu.as_ref().map(|m| m.as_hmenu()),
            hinstance,
            None,
        )?;
        let handle = WindowHandle::new(hwnd);
        let dark_mode = BOOL::from(is_system_dark_mode());
        if dark_mode.as_bool() {
            info!("Dark mode");
        } else {
            info!("Light mode");
        }
        if props.set_attr {
            let ret = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark_mode as *const BOOL as *const std::ffi::c_void,
                std::mem::size_of::<BOOL>() as u32,
            );
            if let Err(e) = ret {
                error!("DwmSetWindowAttribute DWMWA_USE_IMMERSIVE_DARK_MODE: {e}");
            }
        }
        let imm_context = ime::ImmContext::new(handle);
        if props.enable_ime {
            imm_context.enable();
        } else {
            imm_context.disable();
        }
        if let Some(icon) = props.icon {
            if let Ok(big) = icon.load(hinstance) {
                PostMessageW(
                    Some(hwnd),
                    WM_SETICON,
                    WPARAM(ICON_BIG as usize),
                    LPARAM(big.0 as isize),
                )
                .ok();
            }
            if let Ok(small) = icon.load_small(hinstance) {
                PostMessageW(
                    Some(hwnd),
                    WM_SETICON,
                    WPARAM(ICON_SMALL as usize),
                    LPARAM(small.0 as isize),
                )
                .ok();
            }
        }
        DragAcceptFiles(hwnd, props.accept_drop_files);
        set_preferred_app_mode(APPMODE_ALLOWDARK);
        refresh_immersive_color_policy_state();
        let window_props = WindowProps {
            imm_context,
            visible_ime_candidate_window: props.visible_ime_candidate_window,
            auto_close: props.auto_close,
            cursor: props.cursor,
            parent: props.parent,
            nc_hittest: props.nc_hittest,
            redrawing: false,
            resizing: false,
            minimized: false,
            _menu: props.menu,
            theme_menu: Theme::new(hwnd, &["Menu"]),
            color_mode: props.color_mode,
            color_mode_state: if dark_mode.as_bool() {
                ColorModeState::Dark
            } else {
                ColorModeState::Light
            },
        };
        Context::register_window(f(handle), window_props, props.event_rx_id);
        if props.visiblity {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        Ok(handle)
    }
}

impl<Title, Sz, Sty> WindowBuilder<'_, EventReceiver, Title, Sz, Sty>
where
    Title: Into<String>,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    Sty: Style,
{
    /// Builds a window.
    pub fn build(self) -> Result<Window> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<WindowHandle>>();
        let props = BuilderProps::new(self);
        UiThread::send_task(move || {
            tx.send(create_window(props, |handle| {
                WindowKind::Window(Window { handle })
            }))
            .ok();
        });
        let Ok(ret) = rx.blocking_recv() else {
            return Err(Error::UiThreadClosed);
        };
        Ok(Window { handle: ret? })
    }
}

impl<Title, Sz, Sty> WindowBuilder<'_, AsyncEventReceiver, Title, Sz, Sty>
where
    Title: Into<String>,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    Sty: Style,
{
    /// Builds a window.
    pub async fn build(self) -> Result<AsyncWindow> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<WindowHandle>>();
        let props = BuilderProps::new(self);
        UiThread::send_task(move || {
            tx.send(create_window(props, |handle| {
                WindowKind::Window(Window { handle })
            }))
            .ok();
        });
        let Ok(ret) = rx.await else {
            return Err(Error::UiThreadClosed);
        };
        Ok(AsyncWindow { handle: ret? })
    }
}

impl<Title, Sz> std::future::IntoFuture for WindowBuilder<'_, AsyncEventReceiver, Title, Sz>
where
    Title: Into<String>,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
{
    type Output = Result<AsyncWindow>;
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<WindowHandle>>();
        let props = BuilderProps::new(self);
        UiThread::send_task(move || {
            tx.send(create_window(props, |handle| {
                WindowKind::Window(Window { handle })
            }))
            .ok();
        });
        Box::pin(async move {
            let Ok(ret) = rx.await else {
                return Err(Error::UiThreadClosed);
            };
            Ok(AsyncWindow { handle: ret? })
        })
    }
}

mod methods {
    use windows::Win32::Graphics::Gdi::{RDW_INVALIDATE, RedrawWindow};

    use super::*;

    #[inline]
    pub fn position(handle: WindowHandle) -> oneshot::Receiver<ScreenPosition<i32>> {
        let (tx, rx) = oneshot::channel::<ScreenPosition<i32>>();
        UiThread::send_task(move || {
            let rc = get_window_rect(handle.into());
            tx.send((rc.left, rc.top).into()).ok();
        });
        rx
    }

    #[inline]
    pub fn inner_size(handle: WindowHandle) -> oneshot::Receiver<PhysicalSize<u32>> {
        let (tx, rx) = oneshot::channel::<PhysicalSize<u32>>();
        UiThread::send_task(move || {
            let rc = get_client_rect(handle.into());
            tx.send(PhysicalSize::new(
                (rc.right - rc.left) as u32,
                (rc.bottom - rc.top) as u32,
            ))
            .ok();
        });
        rx
    }

    #[inline]
    pub fn dpi(handle: WindowHandle) -> oneshot::Receiver<u32> {
        let (tx, rx) = oneshot::channel::<u32>();
        UiThread::send_task(move || unsafe {
            let dpi = GetDpiForWindow(handle.as_hwnd());
            tx.send(dpi).ok();
        });
        rx
    }

    #[inline]
    pub fn enable_ime(handle: WindowHandle, enabled: bool) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel::<()>();
        UiThread::send_task(move || {
            Context::set_window_props(handle, |props| {
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
    pub fn set_position<T>(handle: WindowHandle, position: T)
    where
        T: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    {
        UiThread::send_task(move || unsafe {
            let dpi = GetDpiForWindow(handle.as_hwnd()) as i32;
            let position = position.to_physical(dpi);
            SetWindowPos(
                handle.as_hwnd(),
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
    pub fn set_size<T>(handle: WindowHandle, size: T)
    where
        T: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        UiThread::send_task(move || unsafe {
            let dpi = GetDpiForWindow(handle.as_hwnd());
            let size = size.to_physical(dpi);
            SetWindowPos(
                handle.as_hwnd(),
                None,
                0,
                0,
                size.width as i32,
                size.height as i32,
                SWP_NOZORDER | SWP_NOMOVE,
            )
            .ok();
        });
    }

    #[inline]
    pub fn close(handle: WindowHandle) {
        unsafe {
            PostMessageW(Some(handle.as_hwnd()), WM_CLOSE, WPARAM(0), LPARAM(0)).ok();
        }
    }

    #[inline]
    pub fn show(handle: WindowHandle) {
        unsafe {
            let _ = ShowWindowAsync(handle.as_hwnd(), SW_SHOW);
        }
    }

    #[inline]
    pub fn hide(handle: WindowHandle) {
        unsafe {
            let _ = ShowWindowAsync(handle.as_hwnd(), SW_HIDE);
        }
    }

    #[inline]
    pub fn minimize(handle: WindowHandle) {
        unsafe {
            let _ = ShowWindowAsync(handle.as_hwnd(), SW_MINIMIZE);
        }
    }

    #[inline]
    pub fn maximize(handle: WindowHandle) {
        unsafe {
            let _ = ShowWindowAsync(handle.as_hwnd(), SW_SHOWMAXIMIZED);
        }
    }

    #[inline]
    pub fn restore(handle: WindowHandle) {
        unsafe {
            let _ = ShowWindowAsync(handle.as_hwnd(), SW_RESTORE);
        }
    }

    #[inline]
    pub fn set_cursor(handle: WindowHandle, cursor: Cursor) {
        UiThread::send_task(move || {
            cursor.set();
            Context::set_window_props(handle, |props| {
                props.cursor = cursor;
            });
        });
    }

    #[inline]
    pub fn redraw(handle: WindowHandle, invalidate_rect: Option<PhysicalRect<i32>>) {
        UiThread::send_task(move || unsafe {
            let rc: Option<RECT> = invalidate_rect.map(|rc| rc.into());
            let p = rc.as_ref().map(|p| p as *const RECT);
            let redrawing = Context::get_window_props(handle, |props| props.redrawing);
            if !redrawing.unwrap_or(true) {
                let _ = RedrawWindow(Some(handle.as_hwnd()), p, None, RDW_INVALIDATE);
                Context::set_window_props(handle, |props| {
                    props.redrawing = true;
                });
            }
        });
    }

    #[inline]
    pub fn post_app_event(handle: WindowHandle, app: event::App) {
        unsafe {
            PostMessageW(
                Some(handle.as_hwnd()),
                OFFSET_WM_APP + app.index,
                WPARAM(app.value0),
                LPARAM(app.value1),
            )
            .ok();
        }
    }

    #[inline]
    pub fn set_foreground(handle: WindowHandle) {
        unsafe {
            let _ = SetForegroundWindow(handle.as_hwnd());
        }
    }

    #[inline]
    pub fn set_focus(handle: WindowHandle) {
        unsafe {
            let _ = SetFocus(Some(handle.as_hwnd()));
        }
    }

    #[inline]
    pub fn color_mode(handle: WindowHandle) -> Option<ColorMode> {
        Context::get_window_props(handle, |props| props.color_mode)
    }

    #[inline]
    pub fn set_color_mode(handle: WindowHandle, mode: ColorMode) {
        UiThread::send_task(move || {
            procedure::change_color_mode(handle.as_hwnd(), mode);
        });
    }

    #[inline]
    pub fn add_raw_procedure_handler<F>(handle: WindowHandle, f: F)
    where
        F: Fn(u32, WPARAM, LPARAM) + Send + 'static,
    {
        UiThread::send_task(move || {
            procedure::add_raw_procedure_handler(handle, f);
        });
    }
}

/// Represents a window.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Window {
    handle: WindowHandle,
}

impl Window {
    #[inline]
    pub fn builder<T>(event_rx: &T) -> WindowBuilder<'_, T> {
        WindowBuilder::new(event_rx)
    }

    #[inline]
    pub fn position(&self) -> Option<ScreenPosition<i32>> {
        let rx = methods::position(self.window_handle());
        rx.blocking_recv().ok()
    }

    #[inline]
    pub fn inner_size(&self) -> Option<PhysicalSize<u32>> {
        let rx = methods::inner_size(self.window_handle());
        rx.blocking_recv().ok()
    }

    #[inline]
    pub fn dpi(&self) -> Option<u32> {
        let rx = methods::dpi(self.window_handle());
        rx.blocking_recv().ok()
    }

    #[inline]
    pub fn set_position<T>(&self, position: T)
    where
        T: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    {
        methods::set_position(self.window_handle(), position);
    }

    #[inline]
    pub fn set_inner_size<T>(&self, size: T)
    where
        T: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        methods::set_size(self.window_handle(), size);
    }

    #[inline]
    pub fn enable_ime(&self, enabled: bool) {
        methods::enable_ime(self.window_handle(), enabled)
            .blocking_recv()
            .ok();
    }

    #[inline]
    pub fn show(&self) {
        methods::show(self.window_handle());
    }

    #[inline]
    pub fn hide(&self) {
        methods::hide(self.window_handle());
    }

    #[inline]
    pub fn minimize(&self) {
        methods::minimize(self.window_handle());
    }

    #[inline]
    pub fn maximize(&self) {
        methods::maximize(self.window_handle());
    }

    #[inline]
    pub fn restore(&self) {
        methods::restore(self.window_handle());
    }

    #[inline]
    pub fn cursor(&self) -> Option<Cursor> {
        Context::get_window_props(self.window_handle(), |props| props.cursor.clone())
    }

    #[inline]
    pub fn set_cursor(&self, cursor: Cursor) {
        let hwnd = self.window_handle();
        methods::set_cursor(hwnd, cursor);
    }

    #[inline]
    pub fn redraw(&self, invalidate_rect: Option<PhysicalRect<i32>>) {
        methods::redraw(self.window_handle(), invalidate_rect);
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        Context::window_is_none(self.window_handle())
    }

    #[inline]
    pub fn close(&self) {
        methods::close(self.window_handle());
    }

    #[inline]
    pub fn post_app_event(&self, app: event::App) {
        methods::post_app_event(self.window_handle(), app);
    }

    #[inline]
    pub fn set_foreground(&self) {
        methods::set_foreground(self.window_handle());
    }

    #[inline]
    pub fn set_focus(&self) {
        methods::set_focus(self.window_handle());
    }

    #[inline]
    pub fn color_mode(&self) -> Option<ColorMode> {
        methods::color_mode(self.window_handle())
    }

    #[inline]
    pub fn set_color_mode(&self, mode: ColorMode) {
        methods::set_color_mode(self.window_handle(), mode);
    }

    #[inline]
    pub fn add_raw_procedure_handler<F>(&self, f: F)
    where
        F: Fn(u32, WPARAM, LPARAM) + Send + 'static
    {
        methods::add_raw_procedure_handler(self.window_handle(), f);
    }

    #[inline]
    pub fn raw_handle(&self) -> *mut std::ffi::c_void {
        self.window_handle().as_hwnd().0
    }
}

impl IsWindow for Window {
    #[inline]
    fn window_handle(&self) -> WindowHandle {
        self.handle
    }
}

/// Represents a window of async version.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AsyncWindow {
    handle: WindowHandle,
}

impl AsyncWindow {
    #[inline]
    pub fn builder(event_rx: &AsyncEventReceiver) -> WindowBuilder<'_, AsyncEventReceiver> {
        WindowBuilder::new(event_rx)
    }

    #[inline]
    pub async fn position(&self) -> Option<ScreenPosition<i32>> {
        let rx = methods::position(self.window_handle());
        rx.await.ok()
    }

    #[inline]
    pub async fn inner_size(&self) -> Option<PhysicalSize<u32>> {
        let rx = methods::inner_size(self.window_handle());
        rx.await.ok()
    }

    #[inline]
    pub async fn dpi(&self) -> Option<u32> {
        let rx = methods::dpi(self.window_handle());
        rx.await.ok()
    }

    #[inline]
    pub fn set_position<T>(&self, position: T)
    where
        T: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    {
        methods::set_position(self.window_handle(), position);
    }

    #[inline]
    pub fn set_inner_size<T>(&self, size: T)
    where
        T: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        methods::set_size(self.window_handle(), size);
    }

    #[inline]
    pub async fn enable_ime(&self, enabled: bool) {
        methods::enable_ime(self.window_handle(), enabled)
            .await
            .ok();
    }

    #[inline]
    pub fn show(&self) {
        methods::show(self.window_handle());
    }

    #[inline]
    pub fn hide(&self) {
        methods::hide(self.window_handle());
    }

    #[inline]
    pub fn minimize(&self) {
        methods::minimize(self.window_handle());
    }

    #[inline]
    pub fn maximize(&self) {
        methods::maximize(self.window_handle());
    }

    #[inline]
    pub fn restore(&self) {
        methods::restore(self.window_handle());
    }

    #[inline]
    pub fn cursor(&self) -> Option<Cursor> {
        Context::get_window_props(self.window_handle(), |props| props.cursor.clone())
    }

    #[inline]
    pub fn set_cursor(&self, cursor: Cursor) {
        let hwnd = self.window_handle();
        methods::set_cursor(hwnd, cursor);
    }

    #[inline]
    pub fn redraw(&self, invalidate_rect: Option<PhysicalRect<i32>>) {
        methods::redraw(self.window_handle(), invalidate_rect);
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        Context::window_is_none(self.window_handle())
    }

    #[inline]
    pub fn close(&self) {
        methods::close(self.window_handle());
    }

    #[inline]
    pub fn post_app_event(&self, app: event::App) {
        methods::post_app_event(self.window_handle(), app);
    }

    #[inline]
    pub fn set_foreground(&self) {
        methods::set_foreground(self.window_handle());
    }

    #[inline]
    pub fn set_focus(&self) {
        methods::set_focus(self.window_handle());
    }

    #[inline]
    pub fn color_mode(&self) -> Option<ColorMode> {
        methods::color_mode(self.window_handle())
    }

    #[inline]
    pub fn set_color_mode(&self, mode: ColorMode) {
        methods::set_color_mode(self.window_handle(), mode);
    }

    #[inline]
    pub fn add_raw_procedure_handler<F>(&self, f: F)
    where
        F: Fn(u32, WPARAM, LPARAM) + Send + 'static,
    {
        methods::add_raw_procedure_handler(self.window_handle(), f);
    }

    #[inline]
    pub fn raw_handle(&self) -> *mut std::ffi::c_void {
        self.window_handle().as_hwnd().0
    }
}

impl IsWindow for AsyncWindow {
    #[inline]
    fn window_handle(&self) -> WindowHandle {
        self.handle
    }
}

fn to_raw_window_handle(
    this: &impl IsWindow,
) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
    use raw_window_handle::{RawWindowHandle, Win32WindowHandle, WindowHandle};
    use std::num::NonZero;
    let this = this.window_handle();
    unsafe {
        let mut handle = Win32WindowHandle::new(
            std::mem::transmute::<usize, std::num::NonZeroIsize>(this.0.0.addr()),
        );
        handle.hinstance = NonZero::new(GetWindowLongPtrW(this.as_hwnd(), GWLP_HINSTANCE));
        Ok(WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)))
    }
}

fn to_raw_display_handle(
    _this: &impl IsWindow,
) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
    Ok(unsafe {
        raw_window_handle::DisplayHandle::borrow_raw(raw_window_handle::RawDisplayHandle::Windows(
            raw_window_handle::WindowsDisplayHandle::new(),
        ))
    })
}

impl raw_window_handle::HasWindowHandle for Window {
    #[inline]
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        to_raw_window_handle(self)
    }
}

impl raw_window_handle::HasWindowHandle for AsyncWindow {
    #[inline]
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        to_raw_window_handle(self)
    }
}

impl raw_window_handle::HasDisplayHandle for Window {
    #[inline]
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        to_raw_display_handle(self)
    }
}

impl raw_window_handle::HasDisplayHandle for AsyncWindow {
    #[inline]
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        to_raw_display_handle(self)
    }
}

/// Builds a inner window.
///
/// Needs specifying position and size of an inner window.
///
pub struct InnerWindowBuilder<'a, Rx, Pos = (), Sz = ()> {
    event_rx: &'a Rx,
    parent_inner: WindowHandle,
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
            parent_inner: parent.window_handle(),
            position: (),
            size: (),
            visibility: true,
            enable_ime: true,
            visible_ime_candidate_window: true,
            accept_drop_files: false,
            cursor: Context::get_window_props(parent.window_handle(), |props| props.cursor.clone())
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

impl<Pos, Sz> InnerWindowBuilder<'_, EventReceiver, Pos, Sz>
where
    Pos: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
{
    /// Builds an inner window.
    #[inline]
    pub fn build(self) -> Result<InnerWindow> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<WindowHandle>>();
        let parent = self.parent_inner;
        let props = BuilderProps::new_inner(self);
        UiThread::send_task(move || {
            tx.send(create_window(props, |handle| {
                WindowKind::InnerWindow(InnerWindow { parent, handle })
            }))
            .ok();
        });
        let Ok(ret) = rx.blocking_recv() else {
            return Err(Error::UiThreadClosed);
        };
        Ok(InnerWindow {
            parent,
            handle: ret?,
        })
    }
}

impl<Pos, Sz> InnerWindowBuilder<'_, AsyncEventReceiver, Pos, Sz>
where
    Pos: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
{
    /// Builds an inner window of async version.
    #[inline]
    pub async fn build(self) -> Result<AsyncInnerWindow> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<WindowHandle>>();
        let parent = self.parent_inner;
        let props = BuilderProps::new_inner(self);
        UiThread::send_task(move || {
            tx.send(create_window(props, |handle| {
                WindowKind::InnerWindow(InnerWindow { parent, handle })
            }))
            .ok();
        });
        let Ok(ret) = rx.await else {
            return Err(Error::UiThreadClosed);
        };
        Ok(AsyncInnerWindow {
            parent,
            handle: ret?,
        })
    }
}

/// Represents an inner window.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InnerWindow {
    parent: WindowHandle,
    handle: WindowHandle,
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
        let self_rx = methods::position(self.handle);
        let self_pos = self_rx.blocking_recv().ok()?;
        let position = unsafe {
            let mut pt = POINT {
                x: self_pos.x,
                y: self_pos.y,
            };
            let _ = ScreenToClient(self.parent.as_hwnd(), &mut pt);
            PhysicalPosition::new(pt.x, pt.y)
        };
        Some(position)
    }

    #[inline]
    pub fn size(&self) -> Option<PhysicalSize<u32>> {
        let rx = methods::inner_size(self.handle);
        rx.blocking_recv().ok()
    }

    #[inline]
    pub fn dpi(&self) -> Option<u32> {
        let rx = methods::dpi(self.handle);
        rx.blocking_recv().ok()
    }

    #[inline]
    pub fn set_position<T>(&self, position: T)
    where
        T: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    {
        methods::set_position(self.handle, position);
    }

    #[inline]
    pub fn set_size<T>(&self, size: T)
    where
        T: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        methods::set_size(self.handle, size);
    }

    #[inline]
    pub fn enable_ime(&self, enabled: bool) {
        methods::enable_ime(self.handle, enabled)
            .blocking_recv()
            .ok();
    }

    #[inline]
    pub fn cursor(&self) -> Option<Cursor> {
        Context::get_window_props(self.handle, |props| props.cursor.clone())
    }

    #[inline]
    pub fn set_cursor(&self, cursor: Cursor) {
        methods::set_cursor(self.handle, cursor);
    }

    #[inline]
    pub fn color_mode(&self) -> Option<ColorMode> {
        methods::color_mode(self.window_handle())
    }

    #[inline]
    pub fn set_color_mode(&self, mode: ColorMode) {
        methods::set_color_mode(self.window_handle(), mode);
    }

    #[inline]
    pub fn post_app_event(&self, app: event::App) {
        methods::post_app_event(self.handle, app);
    }

    #[inline]
    pub fn add_raw_procedure_handler<F>(&self, f: F)
    where
        F: Fn(u32, WPARAM, LPARAM) + Send + 'static,
    {
        methods::add_raw_procedure_handler(self.window_handle(), f);
    }

    #[inline]
    pub fn raw_handle(&self) -> *mut std::ffi::c_void {
        self.handle.as_hwnd().0
    }
}

/// Represents an inner window of async version.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AsyncInnerWindow {
    parent: WindowHandle,
    handle: WindowHandle,
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
        let self_rx = methods::position(self.handle);
        let self_pos = self_rx.await.ok()?;
        let position = unsafe {
            let mut pt = POINT {
                x: self_pos.x,
                y: self_pos.y,
            };
            let _ = ScreenToClient(self.parent.as_hwnd(), &mut pt);
            PhysicalPosition::new(pt.x, pt.y)
        };
        Some(position)
    }

    #[inline]
    pub async fn size(&self) -> Option<PhysicalSize<u32>> {
        let rx = methods::inner_size(self.handle);
        rx.await.ok()
    }

    #[inline]
    pub async fn dpi(&self) -> Option<u32> {
        let rx = methods::dpi(self.handle);
        rx.await.ok()
    }

    #[inline]
    pub fn set_position<T>(&self, position: T)
    where
        T: ToPhysical<i32, Output<i32> = PhysicalPosition<i32>> + Send + 'static,
    {
        methods::set_position(self.handle, position);
    }

    #[inline]
    pub fn set_size<T>(&self, size: T)
    where
        T: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        methods::set_size(self.handle, size);
    }

    #[inline]
    pub fn enable_ime(&self, enabled: bool) {
        methods::enable_ime(self.handle, enabled)
            .blocking_recv()
            .ok();
    }

    #[inline]
    pub fn cursor(&self) -> Option<Cursor> {
        Context::get_window_props(self.handle, |props| props.cursor.clone())
    }

    #[inline]
    pub fn set_cursor(&self, cursor: Cursor) {
        methods::set_cursor(self.handle, cursor);
    }

    #[inline]
    pub fn post_app_event(&self, app: event::App) {
        methods::post_app_event(self.handle, app);
    }

    #[inline]
    pub fn color_mode(&self) -> Option<ColorMode> {
        methods::color_mode(self.window_handle())
    }

    #[inline]
    pub fn set_color_mode(&self, mode: ColorMode) {
        methods::set_color_mode(self.window_handle(), mode);
    }

    #[inline]
    pub fn add_raw_procedure_handler<F>(&self, f: F)
    where
        F: Fn(u32, WPARAM, LPARAM) + Send + 'static,
    {
        methods::add_raw_procedure_handler(self.window_handle(), f);
    }

    #[inline]
    pub fn raw_handle(&self) -> *mut std::ffi::c_void {
        self.handle.as_hwnd().0
    }
}

impl IsWindow for InnerWindow {
    fn window_handle(&self) -> WindowHandle {
        self.handle
    }
}

impl IsWindow for AsyncInnerWindow {
    fn window_handle(&self) -> WindowHandle {
        self.handle
    }
}

impl raw_window_handle::HasWindowHandle for InnerWindow {
    #[inline]
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        to_raw_window_handle(self)
    }
}

impl raw_window_handle::HasWindowHandle for AsyncInnerWindow {
    #[inline]
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        to_raw_window_handle(self)
    }
}

impl raw_window_handle::HasDisplayHandle for InnerWindow {
    #[inline]
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        to_raw_display_handle(self)
    }
}

impl raw_window_handle::HasDisplayHandle for AsyncInnerWindow {
    #[inline]
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        to_raw_display_handle(self)
    }
}

impl PartialEq<Window> for WindowKind {
    #[inline]
    fn eq(&self, other: &Window) -> bool {
        self.window_handle() == other.window_handle()
    }
}

impl PartialEq<InnerWindow> for WindowKind {
    #[inline]
    fn eq(&self, other: &InnerWindow) -> bool {
        self.window_handle() == other.handle
    }
}

impl PartialEq<WindowKind> for Window {
    #[inline]
    fn eq(&self, other: &WindowKind) -> bool {
        self.window_handle() == other.window_handle()
    }
}

impl PartialEq<WindowKind> for InnerWindow {
    #[inline]
    fn eq(&self, other: &WindowKind) -> bool {
        self.handle == other.window_handle()
    }
}

impl PartialEq<AsyncWindowKind> for AsyncWindow {
    #[inline]
    fn eq(&self, other: &AsyncWindowKind) -> bool {
        self.window_handle() == other.window_handle()
    }
}

impl PartialEq<AsyncWindowKind> for AsyncInnerWindow {
    #[inline]
    fn eq(&self, other: &AsyncWindowKind) -> bool {
        self.handle == other.window_handle()
    }
}

impl PartialEq<AsyncWindow> for AsyncWindowKind {
    #[inline]
    fn eq(&self, other: &AsyncWindow) -> bool {
        self.window_handle() == other.window_handle()
    }
}

impl PartialEq<AsyncInnerWindow> for AsyncWindowKind {
    #[inline]
    fn eq(&self, other: &AsyncInnerWindow) -> bool {
        self.window_handle() == other.handle
    }
}
