use super::{Asg, TypeKind};
use crate::{
    ast::{self, Privacy},
    source_files::Source,
};

#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub kind: TypeKind,
    pub source: Source,
    pub privacy: Privacy,
}

impl TypeDecl {
    pub fn num_parameters(&self, asg: &Asg) -> usize {
        self.kind.num_target_parameters(asg)
    }
}

pub type TypeParams = ast::TypeParams;
