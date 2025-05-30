use crate::Show;
use source_files::{Source, SourceFiles};
use std::cmp::Ordering;

#[derive(Debug)]
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

    pub fn cmp_with(&self, other: &Self, source_files: &SourceFiles) -> Ordering {
        self.message.cmp(&other.message).then_with(|| {
            if self.source.is_none() && other.source.is_none() {
                return Ordering::Equal;
            }

            if self.source.is_none() && other.source.is_some() {
                return Ordering::Less;
            }

            if self.source.is_some() && other.source.is_none() {
                return Ordering::Greater;
            }

            let a = self.source.unwrap();
            let b = other.source.unwrap();

            let filename_ordering = source_files
                .get(a.key)
                .filename()
                .cmp(source_files.get(b.key).filename());

            filename_ordering.then_with(|| a.location.cmp(&b.location))
        })
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
