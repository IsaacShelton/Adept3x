mod kind;

use super::Type;
use crate::source_files::Source;
pub use kind::*;

#[derive(Clone, Debug)]
pub struct Destination {
    pub kind: DestinationKind,
    pub resolved_type: Type,
    pub source: Source,
}

impl Destination {
    pub fn new(kind: DestinationKind, resolved_type: Type, source: Source) -> Self {
        Self {
            kind,
            source,
            resolved_type,
        }
    }
}
