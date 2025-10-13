fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let file_menu = wiard::Menu::new()?;
    let menu_index_quit = file_menu.push("quit(&Q)")?;
    let menu = wiard::Menu::new()?;
    let menu_index_item = menu.push("item")?;
    let color_menu = wiard::Menu::new()?;
    let color_menu_system = color_menu.push("system")?;
    let color_menu_light = color_menu.push("light")?;
    let color_menu_dark = color_menu.push("dark")?;
    menu.push(
        wiard::MenuItem::builder()
            .text("Color")
            .sub_menu(&color_menu),
    )?;
    let header_menu = wiard::MenuBar::new()?;
    header_menu.push(
        wiard::MenuBarItem::builder()
            .text("File(&F)")
            .sub_menu(&file_menu),
    )?;
    header_menu.push(wiard::MenuBarItem::builder().text("Menu").sub_menu(&menu))?;
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
            } else if mc.handle == color_menu {
                if mc.index == color_menu_system {
                    window.set_color_mode(wiard::ColorMode::System);
                } else if mc.index == color_menu_light {
                    window.set_color_mode(wiard::ColorMode::Light);
                } else if mc.index == color_menu_dark {
                    window.set_color_mode(wiard::ColorMode::Dark);
                }
            }
        } else if let wiard::Event::ColorModeChanged(ev) = event {
            println!("{ev:?}");
        }
    }
    Ok(())
}
