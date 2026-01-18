use crate::{LineIndex, TextLengthUtf16, TextPointUtf16};
use serde::{Deserialize, Serialize};
use std::{
    iter::Sum,
    ops::{Add, AddAssign},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPointDiffUtf16 {
    pub line: LineIndex,
    pub col: TextLengthUtf16,
}

impl Default for TextPointDiffUtf16 {
    fn default() -> Self {
        Self {
            line: LineIndex(0),
            col: TextLengthUtf16(0),
        }
    }
}

impl TextPointDiffUtf16 {
    pub fn of_str(s: &str) -> Self {
        s.chars().map(Self::of_char).sum()
    }

    pub fn of_char(c: char) -> Self {
        if c == '\n' {
            Self {
                line: LineIndex(1),
                col: TextLengthUtf16(0),
            }
        } else {
            Self {
                line: LineIndex(0),
                col: TextLengthUtf16(c.len_utf16()),
            }
        }
    }
}

impl Add<TextPointDiffUtf16> for TextPointDiffUtf16 {
    type Output = TextPointDiffUtf16;

    // NOTE: Text point addition is not commutative `a + b` != `b + a`
    fn add(self, other: TextPointDiffUtf16) -> Self::Output {
        if other.line == LineIndex(0) {
            Self {
                line: self.line,
                col: self.col + other.col,
            }
        } else {
            Self {
                line: self.line + other.line,
                col: other.col,
            }
        }
    }
}

impl AddAssign for TextPointDiffUtf16 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for TextPointDiffUtf16 {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        return iter.fold(TextPointDiffUtf16::default(), |acc, b| acc + b);
    }
}

impl From<TextPointUtf16> for TextPointDiffUtf16 {
    fn from(value: TextPointUtf16) -> Self {
        Self {
            line: value.line,
            col: value.col,
        }
    }
}
