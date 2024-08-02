use super::{SourceFileKey, SourceFiles};
use crate::line_column::Location;

// WARNING: Don't implement PartialEq, Eq, or Hash for this.
// It's too easy to accidentally define constructs that are only equal
// depending on source, which is usually not what we want.
#[derive(Copy, Clone, Debug)]
pub struct Source {
    pub key: SourceFileKey,
    pub location: Location,
}

impl Source {
    pub fn new(key: SourceFileKey, location: Location) -> Self {
        Self { key, location }
    }

    pub fn internal() -> Self {
        Self {
            key: SourceFiles::INTERNAL_KEY,
            location: Location { line: 1, column: 1 },
        }
    }

    pub fn is_internal(&self) -> bool {
        self.key == SourceFiles::INTERNAL_KEY
    }

    pub fn shift_column(&self, amount: u32) -> Self {
        Self {
            key: self.key,
            location: self.location.shift_column(amount),
        }
    }
}
