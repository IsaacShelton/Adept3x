use crate::{
    backend::BackendError,
    compiler::Compiler,
    diagnostics::{Diagnostics, WarningDiagnostic},
    target::{Target, TargetArch, TargetArchExt, TargetOs, TargetOsExt},
};
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
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

    let mut args = Vec::with_capacity(32);
    args.push("-o".into());
    args.push(output_binary_filepath.as_os_str().into());

    let infrastructure = compiler
        .options
        .infrastructure
        .as_ref()
        .expect("infrastructure to exist")
        .clone();

    if target.os().is_windows() {
        args.push("--start-group".into());
        args.push(infrastructure.join("to_windows").join("crt2.o").into());
        args.push(infrastructure.join("to_windows").join("crtbegin.o").into());
    }

    args.push(output_object_filepath.as_os_str().into());

    // Add arguments to link against requested filenames
    for (filename, _) in compiler.link_filenames.iter_mut() {
        if is_flag_like(filename) {
            eprintln!("warning: ignoring incorrect link filename '{}'", filename);
        } else {
            args.push(OsString::from(filename));
        }
    }

    // Ensure that not trying to link against frameworks when not targeting macOS
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

    if target.os().is_windows() {
        let to_windows = infrastructure.join("to_windows");
        args.push(to_windows.join("libmsvcrt.a").into());
        args.push(to_windows.join("libmingw32.a").into());
        args.push(to_windows.join("libgcc.a").into());
        args.push(to_windows.join("libgcc_eh.a").into());
        args.push(to_windows.join("libmingwex.a").into());
        args.push(to_windows.join("libkernel32.a").into());
        args.push(to_windows.join("libpthread.a").into());
        args.push(to_windows.join("libadvapi32.a").into());
        args.push(to_windows.join("libshell32.a").into());
        args.push(to_windows.join("libuser32.a").into());
        args.push(to_windows.join("libkernel32.a").into());
        args.push(to_windows.join("crtend.o").into());
        args.push("--end-group".into());
    }

    if target.os().is_mac() {
        args.push("-Wl,-ld_classic".into());
    }

    let linker = if target.is_host() {
        // Link for host platform

        match target.os() {
            None | Some(TargetOs::Windows) => {
                please_manually_link(args, diagnostics);
            }
            Some(TargetOs::Mac | TargetOs::Linux) => PathBuf::from_str("/usr/bin/gcc").unwrap(),
        }
    } else {
        // Link for non-host platform

        match target.os() {
            Some(TargetOs::Windows) => {
                let host_os = TargetOs::HOST;
                let host_arch = TargetArch::HOST;

                if (host_os.is_mac() && host_arch.is_aarch64())
                    || (host_os.is_linux() && host_arch.is_x86_64())
                {
                    infrastructure
                        .join("to_windows")
                        .join(&format!("from_{}_{}", host_arch.unwrap(), host_os.unwrap()))
                        .join("x86_64-w64-mingw32-ld")
                } else {
                    please_manually_link(args, diagnostics);
                }
            }
            Some(TargetOs::Mac | TargetOs::Linux) | None => {
                please_manually_link(args, diagnostics);
            }
        }
    };

    // Invoke linker
    let mut command = Command::new(linker)
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
