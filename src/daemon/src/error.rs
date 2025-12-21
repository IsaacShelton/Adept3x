use thiserror::Error;

#[derive(Error, Debug)]
pub enum StartError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to start daemon")]
    FailedToStart,
}
