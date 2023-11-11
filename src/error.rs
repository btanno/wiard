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

impl Error {
    pub(crate) fn from_win32() -> Self {
        windows::core::Error::from_win32().into()
    }
}

pub type Result<T> = ::core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum TryRecvError {
    #[error("Empty")]
    Empty,
    #[error("Disconnected")]
    Disconnected,
}

pub type TryRecvResult<T> = ::core::result::Result<T, TryRecvError>;

