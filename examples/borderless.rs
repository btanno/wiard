fn main() -> anyhow::Result<()> {
    println!("quit to press \"Q\" key");
    let event_rx = wiard::EventReceiver::new();
    let window = wiard::Window::builder(&event_rx)
        .title("wiard borderless")
        .style(wiard::WindowStyle::borderless())
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::KeyInput(k) => {
                if k.is(wiard::VirtualKey::Q, wiard::KeyState::Released) {
                    window.close();
                }
            }
            _ => {}
        }
    }
    Ok(())
}
