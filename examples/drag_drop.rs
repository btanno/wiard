fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard drag drop")
        .build()?;
    let mut effect = wiard::drag_drop::Effect::COPY;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::DragEnter(mut ev) => {
                ev.effect = effect;
                println!("DragEnter: {ev:?}");
            }
            wiard::Event::DragOver(mut ev) => {
                ev.effect = effect;
            }
            wiard::Event::DragLeave => {
                println!("DragLeave");
            }
            wiard::Event::Drop(mut ev) => {
                ev.effect = effect;
                println!("Drop: {ev:?}");
            }
            wiard::Event::KeyInput(ev) => {
                if ev.key_state == wiard::KeyState::Released {
                    match ev.key_code.vkey {
                        wiard::VirtualKey::N => {
                            effect = wiard::drag_drop::Effect::NONE;
                            println!("effect = NONE");
                        }
                        wiard::VirtualKey::C => {
                            effect = wiard::drag_drop::Effect::COPY;
                            println!("effect = COPY");
                        }
                        wiard::VirtualKey::M => {
                            effect = wiard::drag_drop::Effect::MOVE;
                            println!("effect = MOVE");
                        }
                        wiard::VirtualKey::L => {
                            effect = wiard::drag_drop::Effect::LINK;
                            println!("effect = LINK");
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
