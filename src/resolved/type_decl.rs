use super::{Constraint, TypeKind};
use crate::{ast::Privacy, source_files::Source};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub kind: TypeKind,
    pub source: Source,
    pub privacy: Privacy,
}

#[derive(Clone, Debug, Default)]
pub struct TypeParameters {
    pub parameters: HashMap<String, TypeParameter>,
}

impl TypeParameters {
    pub fn len(&self) -> usize {
        self.parameters.len()
    }
}

#[derive(Clone, Debug)]
pub struct TypeParameter {
    pub constraints: Vec<Constraint>,
}
