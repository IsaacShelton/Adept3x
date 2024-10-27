pub mod kind;

use crate::source_files::Source;
pub use kind::*;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub source: Source,
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
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.kind.eq(&other.kind)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.kind, f)
    }
}
