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
            kind: TypeKind::Ptr(Box::new(self)),
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
            TypeKind::Ptr(inner) => inner.strip_constraints(),
            TypeKind::Void => (),
            TypeKind::Never => (),
            TypeKind::AnonymousStruct() => todo!("strip_constraints for anonymous struct"),
            TypeKind::AnonymousUnion() => todo!("strip_constraints for anonymous union"),
            TypeKind::AnonymousEnum(_) => (),
            TypeKind::FixedArray(fixed_array) => fixed_array.inner.strip_constraints(),
            TypeKind::FuncPtr(func) => {
                for param in func.params.required.iter_mut() {
                    param.ty.strip_constraints();
                }
                func.return_type.strip_constraints();
            }
            TypeKind::Enum(_, _) => (),
            TypeKind::Structure(_, _, parameters) => {
                for parameter in parameters {
                    parameter.strip_constraints();
                }
            }
            TypeKind::TypeAlias(_, _) => (),
            TypeKind::Polymorph(_, constraints) => {
                constraints.drain(..);
            }
            TypeKind::Trait(_, _, parameters) => {
                for parameter in parameters {
                    parameter.strip_constraints();
                }
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
