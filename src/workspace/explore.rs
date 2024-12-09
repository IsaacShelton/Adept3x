/*
    =========================  workspace/explore.rs  ==========================
    Module for exploring the native filesystem in order to find files of interest,
    and construct a graph for easier whole analysis
    ---------------------------------------------------------------------------
*/

use super::{
    fs::Fs,
    module_file::ModuleFile,
    normal_file::{NormalFile, NormalFileKind},
    NUM_THREADS,
};
use append_only_vec::AppendOnlyVec;
use ignore::{WalkBuilder, WalkState};
use path_absolutize::Absolutize;
use std::{
    ffi::OsStr,
    fs::FileType,
    path::Path,
    sync::atomic::{self, AtomicBool},
    time::UNIX_EPOCH,
};

pub struct ExploreResult {
    pub normal_files: Vec<NormalFile>,
    pub module_files: Vec<ModuleFile>,
}

pub fn explore(fs: &Fs, folder_path: &Path) -> Result<ExploreResult, ()> {
    let normal_files = AppendOnlyVec::new();
    let module_files = AppendOnlyVec::new();

    let folder_path = folder_path
        .absolutize()
        .expect("failed to get absolute path");

    let walker = WalkBuilder::new(folder_path)
        .threads(NUM_THREADS)
        .standard_filters(false)
        .hidden(true) // Ignore hidden files
        .build_parallel();

    let ok = AtomicBool::new(true);

    walker.run(|| {
        let normal_files = &normal_files;
        let module_files = &module_files;
        let ok = &ok;

        Box::new(move |entry| {
            let Ok(entry) = entry else {
                ok.store(false, atomic::Ordering::SeqCst);
                return WalkState::Quit;
            };

            let basename = entry.file_name();
            let is_file = entry.file_type().as_ref().map_or(false, FileType::is_file);

            // We only care about files
            if !is_file {
                return WalkState::Continue;
            }

            let last_modified_ms = u64::try_from(
                entry
                    .metadata()
                    .unwrap()
                    .modified()
                    .unwrap()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
            )
            .expect("you aren't living millions of years into the future");

            if basename == "_.adept" {
                let fs_node_id = fs
                    .insert(entry.path(), Some(last_modified_ms))
                    .expect("inserted");
                module_files.push(ModuleFile::new(fs_node_id, entry.path().into()));
                return WalkState::Continue;
            }

            let kind = match entry.path().extension().and_then(OsStr::to_str) {
                Some("adept") => NormalFileKind::Adept,
                Some("c") => NormalFileKind::CSource,
                Some("h") => NormalFileKind::CHeader,
                _ => return WalkState::Continue,
            };

            let fs_node_id = fs
                .insert(entry.path(), Some(last_modified_ms))
                .expect("inserted");

            normal_files.push(NormalFile::new(kind, fs_node_id, entry.into_path()));

            WalkState::Continue
        })
    });

    ok.load(atomic::Ordering::SeqCst)
        .then(|| ExploreResult {
            normal_files: normal_files.into_vec(),
            module_files: module_files.into_vec(),
        })
        .ok_or(())
}
