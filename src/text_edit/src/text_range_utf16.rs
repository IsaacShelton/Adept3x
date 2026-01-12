use crate::TextPointUtf16;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPointRangeUtf16 {
    start: TextPointUtf16,
    end: TextPointUtf16,
}

impl TextPointRangeUtf16 {
    #[inline]
    pub fn new(start: TextPointUtf16, end: TextPointUtf16) -> Self {
        Self { start, end }
    }

    #[inline]
    pub fn full(content: &str) -> Self {
        Self::new(TextPointUtf16::start(), TextPointUtf16::end(content))
    }

    #[inline]
    pub fn start(&self) -> TextPointUtf16 {
        self.start
    }

    #[inline]
    pub fn end(&self) -> TextPointUtf16 {
        self.end
    }

    #[inline]
    pub fn encloses(&self, other: &Self) -> bool {
        self.start() <= other.start() && other.end() <= self.end()
    }
}

impl Display for TextPointRangeUtf16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}..{:?}", self.start(), self.end())
    }
}
