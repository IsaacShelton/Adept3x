pub mod kind;

use core::hash::Hash;
use derive_more::Deref;
pub use kind::*;
use primitives::NumericMode;
use source_files::Source;
use std::fmt::Display;

#[derive(Clone, Debug, Deref)]
pub struct Type {
    #[deref]
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

    pub fn numeric_mode(&self) -> Option<NumericMode> {
        match &self.kind {
            TypeKind::Integer(_, sign) => Some(NumericMode::Integer(*sign)),
            TypeKind::CInteger(c_integer, sign) => Some(if let Some(sign) = sign {
                NumericMode::Integer(*sign)
            } else {
                NumericMode::LooseIndeterminateSignInteger(*c_integer)
            }),
            TypeKind::Floating(_) => Some(NumericMode::Float),
            _ => None,
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
