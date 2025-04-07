use fs_tree::FsNodeId;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ModuleFile {
    pub path: PathBuf,
    pub fs_node_id: FsNodeId,
}

impl ModuleFile {
    pub fn new(fs_node_id: FsNodeId, path: PathBuf) -> Self {
        Self { fs_node_id, path }
    }
}
