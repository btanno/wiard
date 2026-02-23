#[test]
#[should_panic]
fn panic_ui_thread_on_main_loop_start() {
    wiard::UiThread::new()
        .on_main_loop_start(|| {
            panic!();
        })
        .build();
}
