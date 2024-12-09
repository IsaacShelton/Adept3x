use super::{
    explore::{explore, ExploreResult},
    fs::{Fs, FsNodeId},
    module_file::ModuleFile,
};
use std::path::{Path, PathBuf};

pub struct ExploreWithinResult {
    pub explored: Option<ExploreResult>,
    pub entry: Option<FsNodeId>,
}

pub fn explore_within(
    fs: &Fs,
    project_folder: &Path,
    single_file: Option<PathBuf>,
) -> ExploreWithinResult {
    if let Some(single_file) = single_file {
        let fs_node_id = fs.insert(&single_file, None).expect("inserted");

        ExploreWithinResult {
            explored: Some(ExploreResult {
                normal_files: vec![],
                module_files: vec![ModuleFile {
                    path: single_file,
                    fs_node_id,
                }],
            }),
            entry: Some(fs_node_id),
        }
    } else {
        ExploreWithinResult {
            explored: explore(fs, project_folder),
            entry: None,
        }
    }
}
