
#[derive(Debug)]
pub struct DataLayerError {
    pub err: Box<dyn std::error::Error + Send + Sync>
}
pub type Result<T> = std::result::Result<T, DataLayerError>;

impl From<sqlx::Error> for DataLayerError {
    fn from(value: sqlx::Error) -> Self {
        Self { err: Box::new(value) }
    }
}