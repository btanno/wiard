fn main() -> anyhow::Result<()> {
    let event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard hello")
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        println!("{event:?}");
    }
    Ok(())
}
