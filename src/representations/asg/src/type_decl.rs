use super::{Asg, TypeKind};
use attributes::Privacy;
use fs_tree::FsNodeId;
use source_files::Source;

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
