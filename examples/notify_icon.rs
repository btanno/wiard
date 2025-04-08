fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let window = wiard::Window::builder(&event_rx)
        .title("wiard notify icon")
        .build()?;
    let _notify_icon = wiard::NotifyIcon::new(&window)
        .icon(&wiard::Icon::from_path("examples/icon.ico"))
        .tip("wiard")
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        if let wiard::Event::NotifyIcon(ev) = event {
            println!("{ev:?}");
        }
    }
    Ok(())
}
