#[tokio::test]
#[should_panic(expected = "panic!!!!!")]
async fn panic_recv_async_test() {
    let mut event_rx = wiard::AsyncEventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .visible(false)
        .build()
        .await
        .unwrap();
    loop {
        tokio::select! {
            ret = event_rx.recv() => { ret.unwrap(); },
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => { panic!("timeout"); },
        }
        wiard::UiThread::send_task(|| {
            panic!("panic!!!!!");
        });
    }
}
