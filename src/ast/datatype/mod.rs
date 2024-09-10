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
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.kind)
    }
}
