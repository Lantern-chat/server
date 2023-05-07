#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database Error: {0}")]
    DbError(#[from] pg::Error),

    #[error("Error recycling connection")]
    RecyclingError,

    #[error("Timeout error {0}")]
    TimeoutError(#[from] tokio::time::error::Elapsed),

    #[error("Timed out")]
    Timeout,

    #[error("Connection Pool is closed")]
    Closed,

    #[error("Could not connect to database")]
    ConnectionFailure,

    #[error("Thorn Format Error: {0}")]
    FormatError(#[from] thorn::macros::SqlFormatError),
}

impl Error {
    pub fn as_db_error(&self) -> Option<&pg::error::DbError> {
        match self {
            Error::DbError(e) => e.as_db_error(),
            _ => None,
        }
    }
}
