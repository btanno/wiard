fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let file_menu = wiard::Menu::new()?;
    let menu_index_quit = file_menu.push(wiard::MenuItem::builder().text("quit(&Q)"))?;
    let menu = wiard::Menu::new()?;
    let menu_index_item = menu.push(wiard::MenuItem::builder().text("item"))?;
    let header_menu = wiard::MenuBar::new()?;
    header_menu.push(
        wiard::MenuBarItem::builder()
            .text("file(&F)")
            .sub_menu(&file_menu),
    )?;
    header_menu.push(wiard::MenuBarItem::builder().text("menu").sub_menu(&menu))?;
    let window = wiard::Window::builder(&event_rx)
        .title("wiard menu")
        .menu(&header_menu)
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        if let wiard::Event::MenuCommand(mc) = event {
            if mc.handle == file_menu {
                if mc.index == menu_index_quit {
                    window.close();
                }
            } else if mc.handle == menu && mc.index == menu_index_item {
                println!("clicked help/item");
            }
        }
    }
    Ok(())
}
