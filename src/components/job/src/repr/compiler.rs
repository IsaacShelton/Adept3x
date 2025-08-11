use source_files::SourceFiles;
use std::path::Path;

// This will be a more limited version of `compiler::Compiler`
// while we transition to the new job system, which we can then remove
// `compiler::Compiler` in favor of this...
pub struct Compiler<'env> {
    pub source_files: &'env SourceFiles,
    pub project_root: Option<&'env Path>,
}

impl<'env> Compiler<'env> {
    pub fn filename<'a>(&self, filename: &'a Path) -> &'a Path {
        self.project_root
            .into_iter()
            .flat_map(|root| filename.strip_prefix(root).ok())
            .next()
            .unwrap_or(filename)
    }
}
