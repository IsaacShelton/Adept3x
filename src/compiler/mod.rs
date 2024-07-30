use crate::{
    cli::BuildOptions, diagnostics::Diagnostics, source_file_cache::SourceFileCache,
    target_info::TargetInfo,
};

pub struct Compiler<'a> {
    pub options: BuildOptions,
    pub target_info: TargetInfo,
    pub source_file_cache: &'a SourceFileCache,
    pub diagnostics: &'a Diagnostics<'a>,
}
