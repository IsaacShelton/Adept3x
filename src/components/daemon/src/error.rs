use std::fmt::Display;

pub enum StartError {
    IoError(std::io::Error),
    FailedToStart,
}

impl From<std::io::Error> for StartError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::IoError(error) => write!(f, "{}", error),
            StartError::FailedToStart => write!(f, "Failed to start daemon"),
        }
    }
}
