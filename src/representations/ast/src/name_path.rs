use itertools::Itertools;
use smallvec::{SmallVec, smallvec};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NamePath {
    pub segments: SmallVec<[Box<str>; 2]>,
}

impl NamePath {
    pub fn new_plain(segment: impl Into<Box<str>>) -> Self {
        Self {
            segments: smallvec![segment.into()],
        }
    }

    pub fn new(segments: SmallVec<[Box<str>; 2]>) -> Self {
        debug_assert!(segments.len() > 0);
        Self { segments }
    }

    pub fn as_plain_str(&self) -> Option<&str> {
        (self.segments.len() == 1).then(|| &*self.segments[0])
    }

    pub fn into_plain(mut self) -> Option<Box<str>> {
        if self.segments.len() == 1 {
            Some(self.segments.drain(..).next().unwrap())
        } else {
            None
        }
    }

    pub fn basename(&self) -> &str {
        self.segments.last().unwrap()
    }

    pub fn has_namespace(&self) -> bool {
        self.segments.len() > 1
    }

    pub fn fullname(&self) -> String {
        Itertools::intersperse(self.segments.iter().map(|x| &**x), "::").collect()
    }
}

impl Display for NamePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, segment) in self.segments.iter().enumerate() {
            if i != 0 {
                write!(f, "::")?;
            }

            write!(f, "{}", segment)?;
        }

        Ok(())
    }
}
