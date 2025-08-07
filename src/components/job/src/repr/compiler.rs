use source_files::SourceFiles;

// This will be a more limited version of `compiler::Compiler`
// while we transition to the new job system, which we can then remove
// `compiler::Compiler` in favor of this...
pub struct Compiler<'env> {
    pub source_files: &'env SourceFiles,
}
