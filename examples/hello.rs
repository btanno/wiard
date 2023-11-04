fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard hello")
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::Closed => println!("closed window"),
            _ => {}
        }
    }
    Ok(())
}
