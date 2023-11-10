fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard uiless ime")
        .visible_ime_candidate_window(false)
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::ImeBeginCandidateList(cl) => {
                println!("ImeBeginCandidateList");
                cl.set_position(wiard::LogicalPosition::new(100, 100));
            }
            wiard::Event::ImeUpdateCandidateList(cl) => {
                println!("ImeUpdateCandidateList");
                println!(
                    "selection = {} : [{}]",
                    cl.selection, cl.items[cl.selection]
                );
                println!("[{}]", cl.items.join(", "));
            }
            wiard::Event::ImeEndCandidateList => {
                println!("ImeEndCandidateList");
            }
            _ => {}
        }
    }
    Ok(())
}
