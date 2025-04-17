fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let window = wiard::Window::builder(&event_rx)
        .title("wiard context menu")
        .build()?;
    let menu = wiard::Menu::new()?;
    menu.push(wiard::MenuItem::builder().text("menu item 0"))?;
    menu.push(wiard::MenuItem::builder().text("menu item 1"))?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::ContextMenu(ev) => {
                println!("{ev:?}");
                menu.track(&window, ev.position)?;
            }
            wiard::Event::MenuCommand(ev) => {
                println!("{ev:?}");
            }
            _ => {}
        }
    }
    Ok(())
}
