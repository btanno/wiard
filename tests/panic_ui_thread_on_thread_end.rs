#[test]
#[should_panic]
fn ui_thread_on_thread_end() {
    wiard::UiThread::new()
        .on_thread_end(|| {
            panic!();
        })
        .build();
    let mut event_rx = wiard::EventReceiver::new();
    let window = wiard::Window::builder(&event_rx)
        .visible(false)
        .build()
        .unwrap();
    window.close();
    loop {
        let Some(_) = event_rx.recv() else {
            break;
        };
    }
    wiard::UiThread::join().unwrap();
}
