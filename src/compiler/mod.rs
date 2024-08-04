use crate::{
    cli::BuildOptions, diagnostics::Diagnostics, source_files::SourceFiles,
    target_info::TargetInfo, version::AdeptVersion,
};
use once_map::OnceMap;
use std::{ffi::OsStr, process::Command, sync::OnceLock};

pub struct Compiler<'a> {
    pub options: BuildOptions,
    pub target_info: &'a TargetInfo,
    pub source_files: &'a SourceFiles,
    pub diagnostics: &'a Diagnostics<'a>,
    pub version: OnceLock<AdeptVersion>,
    pub link_filenames: OnceMap<String, ()>,
    pub link_frameworks: OnceMap<String, ()>,
}

impl<'a> Compiler<'a> {
    pub fn maybe_execute_result(&self, output_binary_filepath: &OsStr) {
        if self.options.excute_result {
            println!("    ==== executing result ====");
            let _ = Command::new(output_binary_filepath)
                .args([] as [&str; 0])
                .spawn()
                .expect("failed to run resulting executable")
                .wait();
        }
    }
}
