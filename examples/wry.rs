use std::cell::RefCell;

thread_local! {
    static WEBVIEW: RefCell<Option<wry::WebView>> = RefCell::new(None);
}

fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let window = wiard::Window::builder(&event_rx)
        .title("wiard with wry")
        .inner_size(wiard::LogicalSize::new(1024, 768))
        .build()?;
    wiard::UiThread::send_task(move || {
        let webview = wry::WebViewBuilder::new()
            .with_url("https://tauri.app")
            .build(&window)
            .unwrap();
        WEBVIEW.with(|wv| {
            *wv.borrow_mut() = Some(webview);
        });
    });
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            _ => {}
        }
    }
    Ok(())
}
