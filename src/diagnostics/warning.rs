use crate::{ast::Source, diagnostics::Diagnostic, show::Show, source_file_cache::SourceFileCache};

pub struct WarningDiagnostic {
    message: String,
    source: Source,
}

impl WarningDiagnostic {
    pub fn new(message: impl ToString, source: Source) -> Self {
        Self {
            message: message.to_string(),
            source,
        }
    }
}

impl Show for WarningDiagnostic {
    fn show(
        &self,
        w: &mut dyn std::fmt::Write,
        source_file_cache: &SourceFileCache,
    ) -> std::fmt::Result {
        write!(
            w,
            "{}:{}:{}: warning: {}",
            source_file_cache.get(self.source.key).filename(),
            self.source.location.line,
            self.source.location.column,
            self.message,
        )
    }
}

impl Diagnostic for WarningDiagnostic {}
