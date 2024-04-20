use tokio::task::JoinError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    JoinError(JoinError),
    FileReadError(tokio::io::Error)
}

impl From<JoinError> for Error {
    fn from(value: JoinError) -> Self {
        Error::JoinError(value)
    }
}

impl From<tokio::io::Error> for Error {
    fn from(value: tokio::io::Error) -> Self {
        Error::FileReadError(value)
    }
}