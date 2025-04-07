mod fixed_array;
mod func_ptr;
mod generics;
mod kind;
mod nameless_enumeration;
mod nameless_structure;
mod nameless_union;

pub use fixed_array::*;
pub use func_ptr::*;
pub use generics::*;
pub use kind::*;
pub use nameless_enumeration::*;
pub use nameless_structure::*;
pub use nameless_union::*;
use source_files::Source;
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
        Type::new(TypeKind::Ptr(Box::new(self)), source)
    }

    pub fn contains_polymorph(&self) -> Option<Source> {
        match &self.kind {
            TypeKind::Boolean
            | TypeKind::Integer(_, _)
            | TypeKind::CInteger(_, _)
            | TypeKind::SizeInteger(_)
            | TypeKind::Floating(_) => None,
            TypeKind::Ptr(inner) => inner.contains_polymorph(),
            TypeKind::FixedArray(fixed_array) => fixed_array.ast_type.contains_polymorph(),
            TypeKind::Void | TypeKind::Never => None,
            TypeKind::Named(_, args) => args
                .iter()
                .flat_map(|arg| match arg {
                    TypeArg::Type(ty) => ty.contains_polymorph(),
                    TypeArg::Expr(_) => todo!("ast::Type::contains_polymorph"),
                })
                .next(),
            TypeKind::AnonymousStruct(_) => None,
            TypeKind::AnonymousUnion(_) => None,
            TypeKind::AnonymousEnum(_) => None,
            TypeKind::FuncPtr(func_pointer) => func_pointer
                .parameters
                .iter()
                .flat_map(|param| param.ast_type.contains_polymorph())
                .next()
                .or_else(|| func_pointer.return_type.contains_polymorph()),
            TypeKind::Polymorph(_) => Some(self.source),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.kind)
    }
}
