use crate::{show::Show, source_file_cache::SourceFileCache};

#[derive(Clone, Debug)]
pub struct BackendError {
    pub message: String,
}

impl From<String> for BackendError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for BackendError {
    fn from(message: &str) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Show for BackendError {
    fn show(
        &self,
        w: &mut impl std::fmt::Write,
        _source_file_cache: &SourceFileCache,
    ) -> std::fmt::Result {
        write!(w, "error: {}", self.message)
    }
}
