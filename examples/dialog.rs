fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let window = wiard::Window::builder(&event_rx)
        .title("wiard dialog")
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        if let wiard::Event::KeyInput(k) = event {
            if k.is(wiard::VirtualKey::O, wiard::KeyState::Pressed) {
                let Some(path) = wiard::FileOpenDialog::new(&window).show() else {
                    println!("FileOpenDialog: Cancelled");
                    continue;
                };
                let path = path.to_string_lossy();
                println!("FileOpenDialog: {path}");
            } else if k.is(wiard::VirtualKey::M, wiard::KeyState::Pressed) {
                let Some(path) = wiard::FileOpenDialog::new_multi_select(&window).show() else {
                    println!("FileOpenDialog: Cancelled");
                    continue;
                };
                let paths = path
                    .into_iter()
                    .map(|path| path.to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("FileOpenDialog: [{paths}]");
            } else if k.is(wiard::VirtualKey::S, wiard::KeyState::Pressed) {
                let Some(path) = wiard::FileSaveDialog::new(&window).show() else {
                    println!("FileSaveDialog: Cancelled");
                    continue;
                };
                let path = path.to_string_lossy();
                println!("FileSaveDialog: {path}");
            }
        }
    }
    Ok(())
}
