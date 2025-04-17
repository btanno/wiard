fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let file_menu = wiard::Menu::new()?;
    file_menu.push(wiard::MenuItem::builder().text("quit"))?;
    let menu = wiard::Menu::new()?;
    menu.push(wiard::MenuItem::builder().text("item"))?;
    let header_menu = wiard::HeaderMenu::new()?;
    header_menu.push(wiard::MenuItem::builder().text("file(&F)"))?;
    header_menu.push(wiard::MenuItem::builder().text("menu(&F)"))?;
    header_menu.push(wiard::MenuItem::builder().text("menu").sub_menu(&menu))?;
    let window = wiard::Window::builder(&event_rx)
        .title("wiard menu")
        .menu(&header_menu)
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::MenuCommand(mc) => {
                if mc.handle == file_menu {
                    if mc.index == 0 {
                        window.close();
                    }
                } else if mc.handle == menu {
                    if mc.index == 0 {
                        println!("clicked help/item");
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
