use derive_more::{Add, AddAssign, Sub, SubAssign, Sum};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    start: TextPosition,
    length: TextLength,
}

impl TextRange {
    #[inline]
    pub fn new(start: TextPosition, length: TextLength) -> Self {
        Self { start, length }
    }

    #[inline]
    pub fn full(content: &str) -> Self {
        Self::new(TextPosition(0), TextLength(content.len()))
    }

    #[inline]
    pub fn start(&self) -> TextPosition {
        self.start
    }

    #[inline]
    pub fn end(&self) -> TextPosition {
        self.start + self.length
    }

    #[inline]
    pub fn len(&self) -> TextLength {
        self.length
    }

    #[inline]
    pub fn of<'a>(&self, content: &'a str) -> &'a str {
        &content[self.start().0..self.end().0]
    }

    #[inline]
    pub fn encloses(&self, other: &TextRange) -> bool {
        self.start() <= other.start() && other.end() <= self.end()
    }

    #[inline]
    pub fn encloses_edit(&self, edit: &TextEdit) -> Option<TextRange> {
        if self.encloses(&edit.range) {
            Some(TextRange::new(
                self.start(),
                self.len() - edit.range.len() + TextLength(edit.replace_with.len()),
            ))
        } else {
            None
        }
    }
}

impl Display for TextRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start().0, self.end().0)
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Sum,
    Serialize,
    Deserialize,
)]
pub struct TextLength(pub usize);

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Sum,
    Serialize,
    Deserialize,
)]
pub struct TextPosition(pub usize);

impl TextPosition {
    pub fn before(self, content: &str) -> &str {
        &content[..self.0]
    }

    pub fn after(self, content: &str) -> &str {
        &content[self.0..]
    }
}

impl std::ops::Add<TextLength> for TextPosition {
    type Output = TextPosition;

    fn add(self, rhs: TextLength) -> Self::Output {
        TextPosition(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign<TextLength> for TextPosition {
    fn add_assign(&mut self, rhs: TextLength) {
        self.0 += rhs.0;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TextEdit<'a> {
    pub(crate) range: TextRange,
    pub(crate) replace_with: &'a str,
}
