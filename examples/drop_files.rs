fn main() -> anyhow::Result<()> {
    let event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard drop files")
        .accept_drop_files(true)
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        if let wiard::Event::DropFiles(df) = event {
            println!("{df:?}");
        }
    }
    Ok(())
}
