#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::AsyncEventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard hello_async")
        .await?;
    loop {
        let Some((event, _)) = event_rx.recv().await else {
            break;
        };
        match event {
            wiard::Event::Closed => println!("closed window"),
            _ => {}
        }
    }
    Ok(())
}
