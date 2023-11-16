fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    let mut event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard logger")
        .build()?;
    loop {
        let Some(_) = event_rx.recv() else {
            break;
        };
    }
    Ok(())
}
