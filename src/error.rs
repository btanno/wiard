#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Api(#[from] windows::core::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    pub(crate) fn from_win32() -> Self {
        windows::core::Error::from_win32().into()
    }
}

pub type Result<T> = ::core::result::Result<T, Error>;

