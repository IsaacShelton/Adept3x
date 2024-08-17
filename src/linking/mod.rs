use crate::{
    backend::BackendError,
    compiler::Compiler,
    diagnostics::{Diagnostics, WarningDiagnostic},
    target::{Target, TargetOs, TargetOsExt},
};
use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

pub fn link_result(
    compiler: &mut Compiler,
    target: &Target,
    diagnostics: &Diagnostics,
    output_object_filepath: &Path,
    output_binary_filepath: &Path,
) -> Result<Duration, BackendError> {
    let start_time = Instant::now();

    let mut args = vec![
        output_object_filepath.as_os_str().into(),
        OsString::from("-o"),
        output_binary_filepath.as_os_str().into(),
    ];

    // Add arguments to link against requested filenames
    for (filename, _) in compiler.link_filenames.iter_mut() {
        if is_flag_like(filename) {
            eprintln!("warning: ignoring incorrect link filename '{}'", filename);
        } else {
            args.push(OsString::from(filename));
        }
    }

    // Ensure that not trying to link against frameworks when not targetting macOS
    if !target.os().is_mac() {
        if let Some((framework, _)) = compiler.link_frameworks.read_only_view().iter().next() {
            return Err(BackendError {
                message: format!(
                    "Cannot link against framework '{framework}' when not targeting macOS"
                ),
            });
        }
    }

    // Add arguments to link against requested frameworks
    for (framework, _) in compiler.link_frameworks.iter_mut() {
        args.push(OsString::from("-framework"));
        args.push(OsString::from(framework));
    }

    // Don't try to link unless on host platform
    if !target.is_host() {
        please_manually_link(args, diagnostics);
    }

    // Invoke linker
    match target.os() {
        Some(TargetOs::Mac | TargetOs::Linux) => {
            // Link resulting object file to create executable
            let mut command = Command::new("gcc")
                .args(args)
                .spawn()
                .expect("Failed to link");

            match command.wait() {
                Ok(bad_status) if !bad_status.success() => {
                    return Err(BackendError::plain("Failed to link"));
                }
                Err(_) => {
                    return Err(BackendError::plain("Failed to spawn linker"));
                }
                Ok(_) => (),
            }

            // Return time it took to link
            Ok(start_time.elapsed())
        }
        Some(TargetOs::Windows) | None => please_manually_link(args, diagnostics),
    }
}

fn please_manually_link(args: Vec<OsString>, diagnostics: &Diagnostics) -> ! {
    let args = args.join(OsStr::new(" "));

    diagnostics.push(WarningDiagnostic::plain(
        format!(
            "Automatic linking is not supported yet on your system, please link manually with something like:\n gcc {}",
            args.to_string_lossy()
        )
    ));

    eprintln!("Success, but requires manual linking, exiting with 1");
    std::process::exit(1);
}

fn is_flag_like(string: &str) -> bool {
    string.chars().skip_while(|c| c.is_whitespace()).next() == Some('-')
}
