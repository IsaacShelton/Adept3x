/*
    =========================  workspace/explore.rs  ==========================
    Module for exploring the native filesystem in order to find files of interest,
    and construct a graph for easier whole analysis
    ---------------------------------------------------------------------------
*/

use super::{fs::Fs, normal_file::NormalFile, NUM_THREADS};
use append_only_vec::AppendOnlyVec;
use ignore::{DirEntry, WalkBuilder, WalkState};
use std::{
    ffi::OsStr,
    fs::FileType,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub struct ExploreResult {
    pub normal_files: Vec<NormalFile>,
    pub module_files: Vec<PathBuf>,
}

pub fn explore(fs: &Fs, folder_path: &Path) -> ExploreResult {
    let normal_files = AppendOnlyVec::new();
    let module_files = AppendOnlyVec::new();

    let walker = WalkBuilder::new(folder_path)
        .threads(NUM_THREADS)
        .standard_filters(false)
        .hidden(true) // Ignore hidden files
        .build_parallel();

    walker.run(|| {
        let normal_files = &normal_files;
        let module_files = &module_files;

        Box::new(move |entry| {
            let entry = entry.unwrap();
            let basename = entry.file_name();
            let is_file = entry.file_type().as_ref().map_or(false, FileType::is_file);

            // We only care about files
            if !is_file {
                return WalkState::Continue;
            }

            let last_modified = u64::try_from(
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
                module_files.push(entry.path().into());
                add_to_fs_graph(fs, &entry, last_modified);
                return WalkState::Continue;
            }

            normal_files.push(match entry.path().extension().and_then(OsStr::to_str) {
                Some("adept") => {
                    add_to_fs_graph(fs, &entry, last_modified);
                    NormalFile::adept(entry.into_path())
                }
                Some("c") => {
                    add_to_fs_graph(fs, &entry, last_modified);
                    NormalFile::c_source(entry.into_path())
                }
                Some("h") => {
                    add_to_fs_graph(fs, &entry, last_modified);
                    NormalFile::c_header(entry.into_path())
                }
                _ => return WalkState::Continue,
            });

            WalkState::Continue
        })
    });

    ExploreResult {
        normal_files: normal_files.into_vec(),
        module_files: module_files.into_vec(),
    }
}

fn add_to_fs_graph(fs: &Fs, entry: &DirEntry, last_modified: u64) {
    fs.insert(&normalized_path_segments(entry.path()), last_modified);
}

pub fn normalized_path_segments(path: &Path) -> Vec<&OsStr> {
    let mut total = Vec::new();

    for segment in path.components() {
        use std::path::Component::*;

        match segment {
            Prefix(p) => total.push(p.as_os_str()),
            RootDir => total.clear(),
            CurDir => {}
            ParentDir => {
                total.pop();
            }
            Normal(n) => total.push(n),
        }
    }

    total
}
