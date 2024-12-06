use super::{Ast, Constraint, TypeKind};
use crate::{ast::Privacy, source_files::Source};
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub kind: TypeKind,
    pub source: Source,
    pub privacy: Privacy,
}

impl TypeDecl {
    pub fn num_parameters(&self, resolved_ast: &Ast) -> usize {
        self.kind.num_target_parameters(resolved_ast)
    }
}

#[derive(Clone, Debug, Default)]
pub struct TypeParameters {
    pub parameters: IndexMap<String, TypeParameter>,
}

impl TypeParameters {
    pub fn len(&self) -> usize {
        self.parameters.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &TypeParameter)> {
        self.parameters.iter()
    }

    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.parameters.keys()
    }
}

#[derive(Clone, Debug)]
pub struct TypeParameter {
    pub constraints: Vec<Constraint>,
}
