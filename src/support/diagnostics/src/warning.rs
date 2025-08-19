use crate::{Diagnostic, minimal_filename, show::Show};
use colored::Colorize;
use source_files::{Source, SourceFiles};
use std::path::Path;

pub struct WarningDiagnostic {
    message: String,
    source: Option<Source>,
}

impl WarningDiagnostic {
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

impl Show for WarningDiagnostic {
    fn show(
        &self,
        w: &mut dyn std::fmt::Write,
        source_files: &SourceFiles,
        project_root: Option<&Path>,
    ) -> std::fmt::Result {
        if let Some(source) = self.source {
            write!(
                w,
                "{}:{}:{}: {} {}",
                minimal_filename(source, source_files, project_root),
                source.location.line,
                source.location.column,
                "warning:".yellow().bold(),
                self.message,
            )
        } else {
            write!(w, "{} {}", "warning".yellow().bold(), self.message)
        }
    }
}

impl Diagnostic for WarningDiagnostic {}
