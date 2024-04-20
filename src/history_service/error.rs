use crate::data_layer_error::DataLayerError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    DataLayerError(DataLayerError),
    GlobPatternError(glob::PatternError),
    GlobError(glob::GlobError),
    ConfigError(Box<dyn std::error::Error>)
}

impl From<glob::PatternError> for Error {
    fn from(value: glob::PatternError) -> Self {
        Error::GlobPatternError(value)
    }
}

impl From<glob::GlobError> for Error {
    fn from(value: glob::GlobError) -> Self {
        Error::GlobError(value)
    }
}

impl From<tokio::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::ConfigError(Box::new(value))
    }
}

impl From<DataLayerError> for Error {
    fn from(value: DataLayerError) -> Self {
        Error::DataLayerError(value)
    }
}