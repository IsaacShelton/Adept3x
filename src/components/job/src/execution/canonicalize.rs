use crate::{module_graph::ModuleGraphRef, repr::Compiler};
use diagnostics::ErrorDiagnostic;
use source_files::Source;
use std::path::{Path, PathBuf};

pub fn canonicalize_or_error(
    compiler: Option<&Compiler>,
    filename: &Path,
    source: Option<Source>,
    graph_ref: ModuleGraphRef,
) -> Result<PathBuf, ErrorDiagnostic> {
    if let Ok(canonicalized) = std::fs::canonicalize(filename) {
        return Ok(canonicalized);
    }

    Err(if let Ok(false) = std::fs::exists(filename) {
        ErrorDiagnostic::new_maybe_source(
            format!(
                "The file '{}' doesn't exist",
                compiler
                    .map(|compiler| compiler.filename(filename))
                    .unwrap_or(filename)
                    .to_string_lossy()
            ),
            source,
        )
        .with_postfix(graph_ref.postfix())
    } else {
        ErrorDiagnostic::new_maybe_source(
            format!(
                "Failed to canonicalize filename '{}'",
                compiler
                    .map(|compiler| compiler.filename(filename))
                    .unwrap_or(filename)
                    .to_string_lossy()
            ),
            source,
        )
        .with_postfix(graph_ref.postfix())
    })
}
