use raw_window_handle::HasWindowHandle;
use windows::Win32::{
    Foundation::HWND,
    Graphics::Direct2D::Common::*,
    Graphics::Direct2D::*,
    Graphics::Dxgi::Common::*,
    System::Com::{COINIT_DISABLE_OLE1DDE, COINIT_MULTITHREADED, CoInitializeEx},
};

fn main() -> anyhow::Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED | COINIT_DISABLE_OLE1DDE).ok()?;
    }
    let mut event_rx = wiard::EventReceiver::new();
    let main_window = wiard::Window::builder(&event_rx)
        .title("wiard inner window")
        .build()?;
    let inner_window = wiard::InnerWindow::builder(&event_rx, &main_window)
        .position(wiard::LogicalPosition::new(10, 10))
        .size(wiard::LogicalSize::new(256, 256))
        .build()?;
    let d2d1_factory: ID2D1Factory =
        unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, None)? };
    let render_target = unsafe {
        let size = inner_window.size().unwrap();
        let dpi = inner_window.dpi().unwrap() as f32;
        let raw_window_handle::RawWindowHandle::Win32(handle) =
            inner_window.window_handle().unwrap().as_raw()
        else {
            unreachable!()
        };
        let hwnd = HWND(isize::from(handle.hwnd) as *mut _);
        d2d1_factory.CreateHwndRenderTarget(
            &D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_UNKNOWN,
                },
                dpiX: dpi,
                dpiY: dpi,
                ..Default::default()
            },
            &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd,
                pixelSize: D2D_SIZE_U {
                    width: size.width,
                    height: size.height,
                },
                ..Default::default()
            },
        )?
    };
    loop {
        let Some((event, window)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::Draw(_) => unsafe {
                render_target.BeginDraw();
                let clear_color = D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.3,
                    a: 0.0,
                };
                render_target.Clear(Some(&clear_color));
                render_target.EndDraw(None, None)?;
            },
            wiard::Event::MouseInput(m) => {
                let b = m.button == wiard::MouseButton::Left
                    && m.button_state == wiard::ButtonState::Pressed;
                if b {
                    if window == main_window {
                        println!("main_window");
                    } else if window == inner_window {
                        println!("inner_window");
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
