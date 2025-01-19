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

#[derive(Copy, Debug)]
pub struct Sourced<T> {
    pub inner: T,
    pub source: Source,
}

impl<T> Sourced<T> {
    pub fn new(inner: T, source: Source) -> Self {
        Self { inner, source }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn as_ref(&self) -> Sourced<&T> {
        Sourced::new(&self.inner, self.source)
    }

    pub fn tuple(self) -> (T, Source) {
        (self.inner, self.source)
    }
}

impl<T: Copy> Sourced<T> {
    pub fn value(&self) -> T {
        self.inner
    }
}

impl<T: Clone> Clone for Sourced<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            source: self.source,
        }
    }
}
