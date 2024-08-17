use crate::{
    cli::BuildOptions, diagnostics::Diagnostics, source_files::SourceFiles,
    target_info::TargetInfo, version::AdeptVersion,
};
use once_map::OnceMap;
use std::{ffi::OsStr, process::Command, sync::OnceLock, time::Duration};

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

            for retry_duration in [10, 10, 10, 50, 100, 250].map(Duration::from_millis) {
                match Command::new(output_binary_filepath)
                    .args([] as [&str; 0])
                    .spawn()
                {
                    Ok(mut process) => {
                        let _ = process.wait();
                        return;
                    }
                    Err(_) => {
                        // Try again in few milliseconds
                        std::thread::sleep(retry_duration);
                    }
                }
            }

            eprintln!("error: failed to run resulting executable");
            std::process::exit(1);
        }
    }
}
