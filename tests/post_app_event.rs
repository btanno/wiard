use std::sync::mpsc;

#[test]
fn post_app_event_test() {
    let (tx, rx) = mpsc::channel::<()>();
    let t = std::thread::spawn(move || {
        let mut event_rx = wiard::EventReceiver::new();
        let window = wiard::Window::builder(&event_rx)
            .visible(false)
            .build()
            .unwrap();
        window.post_app_event(wiard::event::App::new(0, 1, 2));
        loop {
            let Some((event, _)) = event_rx.recv() else {
                break;
            };
            match event {
                wiard::Event::App(ev) => {
                    assert!(ev.index == 0);
                    assert!(ev.value0 == 1);
                    assert!(ev.value1 == 2);
                    tx.send(()).ok();
                    window.close();
                }
                _ => {}
            }
        }
    });
    if let Err(mpsc::RecvTimeoutError::Timeout) = rx.recv_timeout(std::time::Duration::from_secs(3))
    {
        panic!("timeout");
    }
    t.join().unwrap();
}
