use std::sync::mpsc;

#[test]
#[should_panic(expected = "panic!!!!!")]
fn panic_recv_test() {
    let (tx, rx) = mpsc::channel::<()>();
    let t = std::thread::spawn(move || {
        let event_rx = wiard::EventReceiver::new();
        let _window = wiard::Window::builder(&event_rx)
            .visible(false)
            .build()
            .unwrap();
        loop {
            let Some(_) = event_rx.recv() else {
                break;
            };
            wiard::UiThread::send_task(|| {
                panic!("panic!!!!!");
            });
        }
        tx.send(()).ok();
    });
    if let Err(mpsc::RecvTimeoutError::Disconnected) =
        rx.recv_timeout(std::time::Duration::from_secs(3))
    {
        std::panic::resume_unwind(t.join().unwrap_err());
    }
}
