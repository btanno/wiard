# wiard

Window handling library for Windows in Rust

## Simple examples

#### sync version
```rust
fn main() {
    let mut event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .build()
        .unwrap();
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        println!("{event:?}");
    }
}
```

#### async version
```rust
#[tokio::main]
async fn main() {
    let mut event_rx = wiard::AsyncEventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .await
        .unwrap();
    loop {
        let Some((event, _)) = event_rx.recv().await else {
            break;
        };
        println!("{event:?}");
    }
}
```

## License

This library is licensed under the [MIT license](LICENSE).

