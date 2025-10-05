use crate::module_graph::ResolvedLinksetEntry;
use append_only_vec::AppendOnlyVec;
use compiler::BuildOptions;
use diagnostics::{Diagnostics, ErrorDiagnostic, WarningDiagnostic};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
    time::{Duration, Instant},
};
use target::{Target, TargetArch, TargetArchExt, TargetOs, TargetOsExt};

pub fn link_result<'env>(
    linksets: &'env AppendOnlyVec<Vec<ResolvedLinksetEntry<'env>>>,
    options: &BuildOptions,
    target: &Target,
    diagnostics: &Diagnostics,
    output_object_filepath: &Path,
    output_binary_filepath: &Path,
) -> Result<Duration, ErrorDiagnostic> {
    let start_time = Instant::now();

    let mut args = Vec::with_capacity(32);
    args.push("-o".into());
    args.push(output_binary_filepath.as_os_str().into());

    let infrastructure = options
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
    flatten_linksets(&mut args, linksets, target)?;

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
            None | Some(TargetOs::Windows) => infrastructure
                .join("to_windows")
                .join("from_x86_64_windows")
                .join("ld.exe"),
            Some(TargetOs::Mac | TargetOs::Linux) => PathBuf::from_str("/usr/bin/gcc").unwrap(),
            Some(TargetOs::FreeBsd) => PathBuf::from_str("/usr/bin/cc").unwrap(),
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
                    return Err(please_manually_link(args, diagnostics));
                }
            }
            Some(TargetOs::Mac | TargetOs::Linux | TargetOs::FreeBsd) | None => {
                return Err(please_manually_link(args, diagnostics));
            }
        }
    };

    // Invoke linker
    let mut command = Command::new(linker)
        .args(args)
        .spawn()
        .expect("Failed to link executable");

    match command.wait() {
        Ok(bad_status) if !bad_status.success() => {
            return Err(ErrorDiagnostic::plain("Failed to link executable"));
        }
        Err(_) => {
            return Err(ErrorDiagnostic::plain("Failed to spawn linker"));
        }
        Ok(_) => (),
    }

    // Return time it took to link
    Ok(start_time.elapsed())
}

fn please_manually_link(args: Vec<OsString>, diagnostics: &Diagnostics) -> ErrorDiagnostic {
    let args = args.join(OsStr::new(" "));

    diagnostics.push(WarningDiagnostic::plain(
        format!(
            "Automatic linking is not supported yet on your system, please link manually with something like:\n gcc {}",
            args.to_string_lossy()
        )
    ));

    ErrorDiagnostic::plain("Success, but requires manual linking, exiting with 1")
}

fn flatten_linksets(
    args: &mut Vec<OsString>,
    linksets: &AppendOnlyVec<Vec<ResolvedLinksetEntry<'_>>>,
    target: &Target,
) -> Result<(), ErrorDiagnostic> {
    #[derive(Default)]
    struct Data<'env> {
        children: HashSet<&'env ResolvedLinksetEntry<'env>>,
    }

    let mut map = HashMap::<&ResolvedLinksetEntry, Data>::new();

    for linkset in linksets.iter() {
        if linkset.len() == 1 {
            let entry = linkset.iter().next().unwrap();
            map.entry(entry).or_insert_with(|| Default::default());
            continue;
        }

        for window in linkset.windows(2) {
            let entry_a = &window[0];
            let entry_b = &window[1];
            let _a = map.entry(entry_a).or_insert_with(|| Default::default());
            let _b = map
                .entry(entry_b)
                .or_insert_with(|| Default::default())
                .children
                .insert(entry_a);
        }
    }

    let mut queue = Vec::new();

    for (entry, data) in map.iter() {
        if data.children.len() == 0 {
            queue.push(*entry);
        }
    }

    while let Some(entry) = queue.pop() {
        match entry {
            ResolvedLinksetEntry::File(filepath) => {
                args.push(filepath.into());
            }
            ResolvedLinksetEntry::Library(library) => {
                args.push(format!("-l{}", library).into());
            }
            ResolvedLinksetEntry::Framework(framework) => {
                if target.os().is_mac() {
                    args.push(OsString::from("-framework"));
                    args.push(OsString::from(framework));
                }
            }
        }

        for (other_entry, other_data) in map.iter_mut() {
            if other_data.children.remove(&entry) && other_data.children.len() == 0 {
                queue.push(*other_entry);
            }
        }

        map.remove(&entry);
    }

    if map.is_empty() {
        Ok(())
    } else {
        Err(ErrorDiagnostic::plain(format!(
            "Circular linkage dependencies, remaining: {}",
            map.keys().map(|entry| format!("`{}`", entry)).join(", ")
        )))
    }
}
