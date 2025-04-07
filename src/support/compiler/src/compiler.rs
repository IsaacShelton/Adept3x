use crate::BuildOptions;
use compiler_version::AdeptVersion;
use diagnostics::Diagnostics;
use once_map::OnceMap;
use source_files::SourceFiles;
use std::{
    borrow::Cow,
    path::{Path, absolute},
    process::Command,
    sync::OnceLock,
    time::Duration,
};
use target::Target;

pub struct Compiler<'a> {
    pub options: BuildOptions,
    pub source_files: &'a SourceFiles,
    pub diagnostics: &'a Diagnostics<'a>,
    pub version: OnceLock<AdeptVersion>,
    pub link_filenames: OnceMap<String, ()>,
    pub link_frameworks: OnceMap<String, ()>,
}

impl<'a> Compiler<'a> {
    pub fn target(&self) -> &Target {
        &self.options.target
    }

    pub fn execute_result(&self, output_binary_filepath: &Path) -> Result<(), ()> {
        if !self.options.execute_result {
            return Ok(());
        }

        println!("    ==== executing result ====");

        // Avoid using a relative filename to invoke the resulting executable
        let output_binary_filepath = if output_binary_filepath.is_relative() {
            let Ok(absolute_filename) = absolute(&output_binary_filepath) else {
                eprintln!(
                    "error: Failed to get absolute filename of resulting executable '{}'",
                    output_binary_filepath.to_string_lossy().as_ref(),
                );
                return Err(());
            };

            Cow::Owned(absolute_filename)
        } else {
            Cow::Borrowed(output_binary_filepath)
        };

        let mut last_error = None;

        for retry_duration in [10, 10, 10, 50, 100, 250].map(Duration::from_millis) {
            match Command::new(output_binary_filepath.as_os_str())
                .args([] as [&str; 0])
                .spawn()
            {
                Ok(mut process) => {
                    let _ = process.wait();
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);

                    // Try again in few milliseconds
                    std::thread::sleep(retry_duration);
                }
            }
        }

        eprintln!(
            "error: failed to run resulting executable '{}' - {}",
            output_binary_filepath.to_string_lossy().as_ref(),
            last_error.unwrap()
        );
        return Err(());
    }
}
