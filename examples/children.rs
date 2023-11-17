fn main() -> anyhow::Result<()> {
    let event_rx = wiard::EventReceiver::new();
    let mut windows = vec![];
    let root = wiard::Window::builder(&event_rx)
        .title(format!("wiard windows[0]"))
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
        match event {
            wiard::Event::Closed => {
                if let Some(i) = windows.iter().position(|w| w == &window) {
                    println!("closed windows[{i}]");
                }
            }
            _ => {}
        }
    }
    Ok(())
}
