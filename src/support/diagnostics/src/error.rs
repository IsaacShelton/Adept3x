use crate::Show;
use source_files::{Source, SourceFiles};

pub struct ErrorDiagnostic {
    message: String,
    source: Option<Source>,
}

impl ErrorDiagnostic {
    pub fn new(message: impl ToString, source: Source) -> Self {
        Self {
            message: message.to_string(),
            source: Some(source),
        }
    }

    pub fn plain(message: impl ToString) -> Self {
        Self {
            message: message.to_string(),
            source: None,
        }
    }
}

impl Show for ErrorDiagnostic {
    fn show(&self, w: &mut dyn std::fmt::Write, source_files: &SourceFiles) -> std::fmt::Result {
        if let Some(source) = self.source {
            write!(
                w,
                "{}:{}:{}: error: {}",
                source_files.get(source.key).filename(),
                source.location.line,
                source.location.column,
                self.message,
            )
        } else {
            write!(w, "error: {}", self.message)
        }
    }
}
