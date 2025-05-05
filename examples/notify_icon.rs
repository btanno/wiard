fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let window = wiard::Window::builder(&event_rx)
        .title("wiard notify icon")
        .build()?;
    let _notify_icon = wiard::NotifyIcon::new(&window)
        .icon(&wiard::Icon::from_path("examples/icon.ico"))
        .tip("wiard")
        .build()?;
    let menu = wiard::Menu::new()?;
    let menu_index_item = menu.push(wiard::MenuItem::builder().text("item"))?;
    let menu_index_quit = menu.push(wiard::MenuItem::builder().text("quit"))?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::MenuCommand(ev) => {
                if ev.handle == menu && ev.index == menu_index_item {
                    println!("clicked menu item");
                } else if ev.handle == menu && ev.index == menu_index_quit {
                    window.close();
                }
            }
            wiard::Event::NotifyIcon(ev) => {
                println!("{ev:?}");
                if let wiard::NotifyIconEvent::ContextMenu(position) = ev.event {
                    window.set_foreground();
                    window.set_focus();
                    menu.track(&window, position)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
