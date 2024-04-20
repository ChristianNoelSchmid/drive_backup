
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
}

impl From<tokio::io::Error> for Error {
    fn from(value: tokio::io::Error) -> Self {
        Error::IOError(value)
    }
}