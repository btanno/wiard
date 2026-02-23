#[test]
#[should_panic]
fn panic_ui_thread_on_thread_start() {
    wiard::UiThread::new()
        .on_thread_start(|| {
            panic!();
        })
        .build();
}
