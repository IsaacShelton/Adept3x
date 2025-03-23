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
