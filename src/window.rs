use crate::*;
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

pub struct EventReceiver {
    tx: Sender<RecvEvent>,
    rx: Receiver<RecvEvent>,
}

impl EventReceiver {
    #[inline]
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self { tx, rx }
    }

    #[inline]
    pub fn recv(&mut self) -> Option<RecvEvent> {
        self.rx.blocking_recv()
    }

    #[inline]
    pub fn try_recv(&mut self) -> Option<RecvEvent> {
        self.rx.try_recv().ok()
    }

    #[inline]
    pub async fn recv_async(&mut self) -> Option<RecvEvent> {
        self.rx.recv().await
    }
}

pub struct WindowBuilder<Title = &'static str, Sz = LogicalSize<u32>> {
    title: Title,
    inner_size: Sz,
    visibility: bool,
}

impl WindowBuilder {
    #[inline]
    pub fn new() -> Self {
        UiThread::init();
        Self {
            title: "",
            inner_size: LogicalSize::new(1024, 768),
            visibility: true,
        }
    }
}

impl Default for WindowBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<Title, Sz> WindowBuilder<Title, Sz> {
    #[inline]
    pub fn title<T>(self, title: T) -> WindowBuilder<T, Sz>
    where
        T: Into<String>,
    {
        WindowBuilder {
            title,
            inner_size: self.inner_size,
            visibility: self.visibility,
        }
    }

    #[inline]
    pub fn inner_size<Coord>(
        self,
        size: Size<u32, Coord>,
    ) -> WindowBuilder<Title, Size<u32, Coord>> {
        WindowBuilder {
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
    event_tx: Sender<RecvEvent>,
}

impl<Sz> BuilderProps<Sz> {
    fn new<Title>(builder: WindowBuilder<Title, Sz>, event_tx: Sender<RecvEvent>) -> Self
    where
        Title: Into<String>,
        Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
    {
        Self {
            title: HSTRING::from(builder.title.into()),
            inner_size: builder.inner_size,
            visiblity: builder.visibility,
            event_tx,
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
        Context::register_window(hwnd, props.event_tx);
        Ok(hwnd)
    }
}

impl<Title, Sz> WindowBuilder<Title, Sz>
where
    Title: Into<String>,
    Sz: ToPhysical<u32, Output<u32> = PhysicalSize<u32>> + Send + 'static,
{
    pub fn build(self, event_rx: &EventReceiver) -> Result<Window> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<HWND>>();
        let props = BuilderProps::new(self, event_rx.tx.clone());
        UiThread::send_task(move || {
            tx.send(create_window(props)).ok();
        });
        let Ok(ret) = rx.blocking_recv() else {
            return Err(Error::UiThreadClosed);
        };
        let hwnd = ret?;
        Ok(Window { hwnd })
    }

    pub async fn build_async(self, event_rx: &EventReceiver) -> Result<Window> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<HWND>>();
        let props = BuilderProps::new(self, event_rx.tx.clone());
        UiThread::send_task(move || {
            tx.send(create_window(props)).ok();
        });
        let Ok(ret) = rx.await else {
            return Err(Error::UiThreadClosed);
        };
        let hwnd = ret?;
        Ok(Window { hwnd })
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
    pub fn builder() -> WindowBuilder {
        WindowBuilder::new()
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
