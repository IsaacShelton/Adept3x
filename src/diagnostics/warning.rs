use crate::{
    diagnostics::Diagnostic,
    show::Show,
    source_files::{Source, SourceFiles},
};

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
        source_file_cache: &SourceFiles,
    ) -> std::fmt::Result {
        if let Some(source) = self.source {
            write!(
                w,
                "{}:{}:{}: warning: {}",
                source_file_cache.get(source.key).filename(),
                source.location.line,
                source.location.column,
                self.message,
            )
        } else {
            write!(w, "warning: {}", self.message,)
        }
    }
}

impl Diagnostic for WarningDiagnostic {}
