use derive_more::IsVariant;
use fs_tree::FsNodeId;
use std::path::PathBuf;

#[derive(Debug, IsVariant)]
pub enum NormalFileKind {
    Adept,
    CSource,
    CHeader,
}

#[derive(Debug)]
pub struct NormalFile {
    pub kind: NormalFileKind,
    pub path: PathBuf,
    pub fs_node_id: FsNodeId,
}

impl NormalFile {
    pub fn new(kind: NormalFileKind, fs_node_id: FsNodeId, path: PathBuf) -> Self {
        Self {
            kind,
            path,
            fs_node_id,
        }
    }
}
