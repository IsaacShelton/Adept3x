mod kind;

use super::Type;
use crate::source_files::Source;
use core::hash::Hash;
pub use kind::*;

#[derive(Clone, Debug)]
pub struct Destination {
    pub kind: DestinationKind,
    pub resolved_type: Type,
    pub source: Source,
}

impl Hash for Destination {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.resolved_type.hash(state);
    }
}

impl PartialEq for Destination {
    fn eq(&self, other: &Self) -> bool {
        self.kind.eq(&other.kind) && self.resolved_type.eq(&other.resolved_type)
    }
}

impl Eq for Destination {}

impl Destination {
    pub fn new(kind: DestinationKind, resolved_type: Type, source: Source) -> Self {
        Self {
            kind,
            source,
            resolved_type,
        }
    }
}
