fn main() -> anyhow::Result<()> {
    let event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .auto_close(false)
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::CloseRequest(cr) => {
                println!("CloseRequest");
                cr.destroy();
            }
            wiard::Event::Closed => {
                println!("Closed");
            }
            _ => {}
        }
    }
    Ok(())
}
