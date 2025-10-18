#![allow(clippy::needless_doctest_main)]

//! Window handing library for Windows in Rust
//!
//! # Simple examples
//!
//! #### sync version
//! ```no_run
//! fn main() {
//!     let mut event_rx = wiard::EventReceiver::new();
//!     let _window = wiard::Window::builder(&event_rx)
//!         .build()
//!         .unwrap();
//!     loop {
//!         let Some((event, _)) = event_rx.recv() else {
//!             break;
//!         };
//!         println!("{event:?}");
//!     }
//! }
//! ```
//!
//! #### async version
//! ```no_run
//! #[tokio::main]
//! async fn main() {
//!     let mut event_rx = wiard::AsyncEventReceiver::new();
//!     let _window = wiard::Window::builder(&event_rx)
//!         .await
//!         .unwrap();
//!     loop {
//!         let Some((event, _)) = event_rx.recv().await else {
//!             break;
//!         };
//!         println!("{event:?}");
//!     }
//! }
//! ```
//!
//! # Note
//! wiard use `WM_APP`. Don't post directly `WM_APP` to wiard's UI thread.
//!

pub use gedv::*;

mod color;
mod context;
mod device;
mod dialog;
mod error;
pub mod event;
pub mod ime;
pub mod menu;
mod messages;
pub mod notify_icon;
mod procedure;
mod resource;
pub mod style;
mod ui_thread;
mod utility;
mod window;

pub use color::*;
use context::*;
pub use device::*;
pub use dialog::*;
pub use dialog::{FileDialogOptions, FileOpenDialog};
pub use error::*;
#[doc(inline)]
pub use event::{Event, NcHitTestValue, ResizingEdge};
#[doc(inline)]
pub use menu::{Menu, MenuBar, MenuBarItem, MenuHandle, MenuItem};
use messages::*;
#[doc(inline)]
pub use notify_icon::*;
pub use resource::*;
#[doc(inline)]
pub use style::*;
pub use ui_thread::UiThread;
use utility::*;
pub use window::*;

pub mod api {
    pub use windows::*;
}