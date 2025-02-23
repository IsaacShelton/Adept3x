use super::{Asg, TypeKind};
use crate::{
    ast::{self, Privacy},
    source_files::Source,
    workspace::fs::FsNodeId,
};

#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub kind: TypeKind,
    pub source: Source,
    pub privacy: Privacy,
    pub file_fs_node_id: FsNodeId,
}

impl TypeDecl {
    pub fn num_parameters(&self, asg: &Asg) -> usize {
        self.kind.num_target_parameters(asg)
    }
}

pub type TypeParams = ast::TypeParams;
