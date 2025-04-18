fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let mut windows = vec![];
    let root = wiard::Window::builder(&event_rx)
        .title("wiard windows[0]".to_string())
        .inner_size(wiard::LogicalSize::new(320, 240))
        .build()?;
    windows.push(root);
    for i in 1..3 {
        let window = wiard::Window::builder(&event_rx)
            .title(format!("wiard children windows[{i}]"))
            .inner_size(wiard::LogicalSize::new(320, 240))
            .parent(windows.last().unwrap())
            .build()?;
        windows.push(window);
    }
    loop {
        let Some((event, window)) = event_rx.recv() else {
            break;
        };
        if let wiard::Event::Closed = event {
            if let Some(i) = windows.iter().position(|w| w == &window) {
                println!("closed windows[{i}]");
            }
        }
    }
    Ok(())
}
