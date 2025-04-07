use super::{
    explore::{ExploreResult, explore},
    module_file::ModuleFile,
};
use fs_tree::{Fs, FsNodeId};
use std::path::{Path, PathBuf};

pub struct ExploreWithinResult {
    pub explored: ExploreResult,
    pub entry: Option<FsNodeId>,
}

pub fn explore_within(
    fs: &Fs,
    project_folder: &Path,
    single_file: Option<PathBuf>,
) -> Result<ExploreWithinResult, ()> {
    Ok(match single_file {
        None => ExploreWithinResult {
            explored: explore(fs, project_folder)?,
            entry: None,
        },
        Some(single_file) => {
            let fs_node_id = fs.insert(&single_file, None).expect("inserted");

            ExploreWithinResult {
                explored: ExploreResult {
                    normal_files: vec![],
                    module_files: vec![ModuleFile {
                        path: single_file,
                        fs_node_id,
                    }],
                },
                entry: Some(fs_node_id),
            }
        }
    })
}
