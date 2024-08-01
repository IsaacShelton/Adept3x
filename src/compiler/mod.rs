use crate::{
    cli::BuildOptions, diagnostics::Diagnostics, source_file_cache::SourceFileCache,
    target_info::TargetInfo, version::AdeptVersion,
};
use once_map::OnceMap;
use std::sync::OnceLock;

pub struct Compiler<'a> {
    pub options: BuildOptions,
    pub target_info: &'a TargetInfo,
    pub source_file_cache: &'a SourceFileCache,
    pub diagnostics: &'a Diagnostics<'a>,
    pub version: OnceLock<AdeptVersion>,
    pub link_filenames: OnceMap<String, ()>,
    pub link_frameworks: OnceMap<String, ()>,
}
