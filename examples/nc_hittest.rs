fn main() -> anyhow::Result<()> {
    let event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard nc_hittest")
        .hook_nc_hittest(true)
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        if let wiard::Event::NcHitTest(t) = event {
            t.set(Some(wiard::NcHitTestValue::Caption));
        }
    }
    Ok(())
}
