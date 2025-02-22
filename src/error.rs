/// The error type for Windows apis, std::io and an UI thread state.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Api(#[from] windows::core::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("UiThreadClosed")]
    UiThreadClosed,
}

/// This type is `Result<T, wiard::Error>`.
pub type Result<T> = ::core::result::Result<T, Error>;

/// The error type for `EventReceiver` and `AsyncEventReceiver`.
#[derive(Debug, thiserror::Error)]
pub enum TryRecvError {
    #[error("Empty")]
    Empty,
    #[error("Disconnected")]
    Disconnected,
}

/// This type is `Result<T, wiard::TryRecvError>`.
pub type TryRecvResult<T> = ::core::result::Result<T, TryRecvError>;

macro_rules! error {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::error!($($arg)*);
        #[cfg(not(feature = "tracing"))]
        { format_args!($($arg)*); }
    };
}

macro_rules! warning {
    ($($arg:tt)+) => {
        #[cfg(feature = "tracing")]
        tracing::warn!($($arg)*);
        #[cfg(not(feature = "tracing"))]
        { format_args!($($arg)*); }
    };
}

macro_rules! info {
    ($($arg:tt)*) => {
        #[cfg(feature = "tracing")]
        tracing::info!($($arg)*);
        #[cfg(not(feature = "tracing"))]
        { format_args!($($arg)*); }
    };
}

pub(crate) use error;
pub(crate) use info;
pub(crate) use warning;
