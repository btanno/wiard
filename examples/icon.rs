fn main() -> anyhow::Result<()> {
    let event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard icon")
        .icon(wiard::Icon::from_path("examples/icon.ico"))
        .build()?;
    loop {
        if event_rx.recv().is_none() {
            break;
        }
    }
    Ok(())
}
