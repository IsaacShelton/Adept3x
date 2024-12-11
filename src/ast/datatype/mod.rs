mod c_integer;
mod description;
mod fixed_array;
mod function_pointer;
mod generics;
mod kind;
mod nameless_enumeration;
mod nameless_structure;
mod nameless_union;

use crate::source_files::Source;
pub use c_integer::*;
pub use description::*;
pub use fixed_array::*;
pub use function_pointer::*;
pub use generics::*;
pub use kind::*;
pub use nameless_enumeration::*;
pub use nameless_structure::*;
pub use nameless_union::*;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub source: Source,
}

impl Type {
    pub fn new(kind: TypeKind, source: Source) -> Self {
        Self { kind, source }
    }

    pub fn pointer(self) -> Self {
        let source = self.source;
        Type::new(TypeKind::Pointer(Box::new(self)), source)
    }

    pub fn contains_polymorph(&self) -> Option<Source> {
        match &self.kind {
            TypeKind::Boolean
            | TypeKind::Integer(_, _)
            | TypeKind::CInteger(_, _)
            | TypeKind::Floating(_) => None,
            TypeKind::Pointer(inner) => inner.contains_polymorph(),
            TypeKind::FixedArray(fixed_array) => fixed_array.ast_type.contains_polymorph(),
            TypeKind::Void => None,
            TypeKind::Named(_, args) => args
                .iter()
                .flat_map(|arg| match arg {
                    CompileTimeArgument::Type(ty) => ty.contains_polymorph(),
                    CompileTimeArgument::Expr(_) => todo!("ast::Type::contains_polymorph"),
                })
                .next(),
            TypeKind::AnonymousStruct(_) => todo!("contains_polymoph for AnonymousStruct"),
            TypeKind::AnonymousUnion(_) => todo!("contains_polymorph for AnonymousUnion"),
            TypeKind::AnonymousEnum(_) => todo!("contains_polymorph for AnonymousEnum"),
            TypeKind::FunctionPointer(_) => todo!("contains_polymorph for FunctionPointer"),
            TypeKind::Polymorph(_, _) => Some(self.source),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.kind)
    }
}
