use diagnostics::ErrorDiagnostic;
use source_files::Source;
use std::path::{Path, PathBuf};

pub fn canonicalize_or_error(
    filename: &Path,
    source: Option<Source>,
) -> Result<PathBuf, ErrorDiagnostic> {
    if let Ok(canonicalized) = std::fs::canonicalize(filename) {
        return Ok(canonicalized);
    }

    Err(if let Ok(false) = std::fs::exists(filename) {
        ErrorDiagnostic::new_maybe_source(
            format!("File does not exist: {}", filename.to_string_lossy()),
            source,
        )
    } else {
        ErrorDiagnostic::new_maybe_source(
            format!(
                "Failed to canonicalize filename: {}",
                filename.to_string_lossy()
            ),
            source,
        )
    })
}
