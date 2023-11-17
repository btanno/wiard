#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let event_rx = wiard::AsyncEventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard hello_async")
        .await?;
    loop {
        let Some((event, _)) = event_rx.recv().await else {
            break;
        };
        println!("{event:?}");
    }
    Ok(())
}
