fn main() -> anyhow::Result<()> {
    let event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard uiless ime")
        .visible_ime_candidate_window(false)
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::ImeBeginComposition(_) => {
                println!("ImeBeginComposition");
            }
            wiard::Event::ImeUpdateComposition(comp) => {
                println!("ImeUpdateComposition: {comp:?}");
            }
            wiard::Event::ImeEndComposition(comp) => {
                println!("ImeEndComposition: {comp:?}");
            }
            wiard::Event::ImeBeginCandidateList => {
                println!("ImeBeginCandidateList");
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
