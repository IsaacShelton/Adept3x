use diagnostics::Show;
use source_files::SourceFiles;

#[derive(Clone, Debug)]
pub struct BackendError {
    pub message: String,
}

impl BackendError {
    pub fn plain(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
        }
    }
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
    fn show(&self, w: &mut dyn std::fmt::Write, _source_files: &SourceFiles) -> std::fmt::Result {
        write!(w, "error: {}", self.message)
    }
}
