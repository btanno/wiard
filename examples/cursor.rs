fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let window = wiard::Window::builder(&event_rx)
        .title("wiard cursor")
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::KeyInput(k) => {
                if k.is(wiard::VirtualKey::A, wiard::KeyState::Pressed) {
                    window.set_cursor(wiard::Cursor::Arrow);
                } else if k.is(wiard::VirtualKey::S, wiard::KeyState::Pressed) {
                    window.set_cursor(wiard::Cursor::Hand);
                } else if k.is(wiard::VirtualKey::D, wiard::KeyState::Pressed) {
                    window.set_cursor(wiard::Cursor::IBeam);
                } else if k.is(wiard::VirtualKey::F, wiard::KeyState::Pressed) {
                    window.set_cursor(wiard::Cursor::Wait);
                }
            }
            _ => {}
        }
    }
    Ok(())
}
