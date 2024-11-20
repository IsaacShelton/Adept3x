pub mod kind;

use crate::source_files::Source;
use core::hash::Hash;
pub use kind::*;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub source: Source,
}

impl Hash for Type {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state)
    }
}

impl Type {
    pub fn pointer(self, source: Source) -> Self {
        Self {
            kind: TypeKind::Pointer(Box::new(self)),
            source,
        }
    }

    pub fn is_ambiguous(&self) -> bool {
        self.kind.is_ambiguous()
    }

    pub fn strip_constraints(&mut self) {
        match &mut self.kind {
            TypeKind::Unresolved => panic!(),
            TypeKind::Boolean => (),
            TypeKind::Integer(_, _) => (),
            TypeKind::CInteger(_, _) => (),
            TypeKind::IntegerLiteral(_) => (),
            TypeKind::FloatLiteral(_) => (),
            TypeKind::Floating(_) => (),
            TypeKind::Pointer(inner) => inner.strip_constraints(),
            TypeKind::Void => (),
            TypeKind::AnonymousStruct() => todo!(),
            TypeKind::AnonymousUnion() => todo!(),
            TypeKind::AnonymousEnum() => todo!(),
            TypeKind::FixedArray(fixed_array) => fixed_array.inner.strip_constraints(),
            TypeKind::FunctionPointer(_) => todo!(),
            TypeKind::Enum(_, _) => (),
            TypeKind::Structure(_, _) => (),
            TypeKind::TypeAlias(_, _) => (),
            TypeKind::Polymorph(_, constraints) => {
                constraints.drain(..);
            }
        }
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.kind.eq(&other.kind)
    }
}

impl Eq for Type {}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.kind, f)
    }
}
