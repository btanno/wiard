use crate::*;
use std::sync::atomic::{self, AtomicU64};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        GetStockObject, MonitorFromPoint, RedrawWindow, HBRUSH, MONITOR_DEFAULTTOPRIMARY,
        RDW_INVALIDATE, WHITE_BRUSH,
    },
    System::LibraryLoader::GetModuleHandleW,
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

pub type RecvEvent = (Event, Window);
pub type AsyncRecvEvent = (Event, AsyncWindow);

fn gen_id() -> u64 {
    static ID: AtomicU64 = AtomicU64::new(0);
    ID.fetch_add(1, atomic::Ordering::SeqCst)
}

pub struct EventReceiver {
    id: u64,
    rx: Receiver<RecvEvent>,
}

impl EventReceiver {
    #[inline]
    pub fn new() -> Self {
        let id = gen_id();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Context::register_event_tx(id, tx);
        Self { id, rx }
    }

    #[inline]
    pub fn recv(&mut self) -> Option<RecvEvent> {
        self.rx.blocking_recv()
    }

    #[inline]
    pub fn try_recv(&mut self) -> Result<Option<RecvEvent>> {
        use tokio::sync::mpsc::error::TryRecvError;

        match self.rx.try_recv() {
            Ok(ret) => Ok(Some(ret)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(Error::UiThreadClosed),
        }
    }
}

pub struct AsyncEventReceiver {
    id: u64,
    rx: Receiver<RecvEvent>,
}

impl AsyncEventReceiver {
    #[inline]
    pub fn new() -> Self {
        let id = gen_id();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Context::register_event_tx(id, tx);
        Self { id, rx }
    }

    #[inline]
    pub async fn recv(&mut self) -> Option<AsyncRecvEvent> {
        let ret = self.rx.recv().await?;
        Some((ret.0, AsyncWindow { hwnd: ret.1.hwnd }))
    }

    #[inline]
    pub fn try_recv(&mut self) -> Result<Option<AsyncRecvEvent>> {
        use tokio::sync::mpsc::error::TryRecvError;

        match self.rx.try_recv() {
            Ok(ret) => Ok(Some((ret.0, AsyncWindow { hwnd: ret.1.hwnd }))),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(Error::UiThreadClosed),
        }
    }
}

pub struct WindowBuilder<'a, Rx, Title = &'static str, Sz = LogicalSize<u32>> {
    event_rx: &'a Rx,
    title: Title,
    inner_size: Sz,
    visibility: bool,
}

impl<'a, Rx> WindowBuilder<'a, Rx> {
    #[inline]
    pub fn new(event_rx: &'a Rx) -> Self {
        UiThread::init();
        Self {
            event_rx,
            title: "",
            inner_size: LogicalSize::new(1024, 768),
            visibility: true,
        }
    }
}

impl<'a, Rx, Title, Sz> WindowBuilder<'a, Rx, Title, Sz> {
    #[inline]
    pub fn title<T>(self, title: T) -> WindowBuilder<'a, Rx, T, Sz>
    where
        T: Into<String>,
    {
        WindowBuilder {
            event_rx: self.event_rx,
            title,
            inner_size: self.inner_size,
            visibility: self.visibility,
        }
    }

    #[inline]
    pub fn inner_size<Coord>(
        self,
        size: Size<u32, Coord>,
    ) -> WindowBuilder<'a, Rx, Title, Size<u32, Coord>> {
        WindowBuilder {
            event_rx: self.event_rx,
            title: self.title,
            inner_size: size,
            visibility: self.visibility,
        }
    }

    #[inline]
    pub fn visible(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }
}

struct BuilderProps<Sz> {
    title: HSTRING,
    inner_size: Sz,
    visiblity: bool,
    event_rx_id: u64,
}

impl<Sz> BuilderProps<Sz> {
    fn new<Rx, Title>(builder: WindowBuilder<Rx, Title, Sz>, event_rx_id: u64) -> Self
    where
        Title: Into<String>,
        Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        Self {
            title: HSTRING::from(builder.title.into()),
            inner_size: builder.inner_size,
            visiblity: builder.visibility,
            event_rx_id,
        }
    }
}

fn create_window<Sz>(props: BuilderProps<Sz>) -> Result<HWND>
where
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
{
    unsafe {
        let dpi = get_dpi_from_point(ScreenPosition::new(0, 0));
        let size = props.inner_size.to_physical(dpi);
        let style = WS_OVERLAPPEDWINDOW;
        let ex_style = WINDOW_EX_STYLE(0);
        let rc = adjust_window_rect_ex_for_dpi(size, style, false, ex_style, dpi);
        let hinstance = GetModuleHandleW(None).unwrap();
        let hwnd = CreateWindowExW(
            ex_style,
            WINDOW_CLASS_NAME,
            &props.title,
            style,
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
        if props.visiblity {
            ShowWindow(hwnd, SW_SHOW);
        }
        Context::register_window(hwnd, props.event_rx_id);
        Ok(hwnd)
    }
}

impl<'a, Title, Sz> WindowBuilder<'a, EventReceiver, Title, Sz>
where
    Title: Into<String>,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
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

impl<'a, Title, Sz> WindowBuilder<'a, AsyncEventReceiver, Title, Sz>
where
    Title: Into<String>,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
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
    pub fn inner_size(&self) -> Option<PhysicalSize<u32>> {
        let (tx, rx) = tokio::sync::oneshot::channel::<PhysicalSize<u32>>();
        let hwnd = self.hwnd.clone();
        UiThread::send_task(move || {
            let rc = get_client_rect(hwnd);
            tx.send(PhysicalSize::new(
                (rc.right - rc.left) as u32,
                (rc.bottom - rc.top) as u32,
            ))
            .ok();
        });
        rx.blocking_recv().ok()
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
    pub async fn inner_size(&self) -> Option<PhysicalSize<u32>> {
        let (tx, rx) = tokio::sync::oneshot::channel::<PhysicalSize<u32>>();
        let hwnd = self.hwnd.clone();
        UiThread::send_task(move || {
            let rc = get_client_rect(hwnd);
            tx.send(PhysicalSize::new(
                (rc.right - rc.left) as u32,
                (rc.bottom - rc.top) as u32,
            ))
            .ok();
        });
        rx.await.ok()
    }
}
