use fs_tree::FsNodeId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeclScopeRef {
    file: FsNodeId,
}

impl DeclScopeRef {
    pub fn new(file: FsNodeId) -> Self {
        Self { file }
    }
}
