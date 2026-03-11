fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard drag drop")
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::DragEnter(mut ev) => {
                ev.effect = wiard::drag_drop::Effect::COPY;
                println!("DragEnter: {ev:?}");
            }
            wiard::Event::DragOver(_ev) => {}
            wiard::Event::DragLeave => {
                println!("DragLeave");
            }
            wiard::Event::Drop(ev) => {
                println!("Drop: {ev:?}");
            }
            _ => {}
        }
    }
    Ok(())
}
