use crate::{LineIndex, TextLengthUtf16, TextPointDiffUtf16};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    ops::{Add, AddAssign},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPointUtf16 {
    pub line: LineIndex,
    pub col: TextLengthUtf16,
}

impl TextPointUtf16 {
    pub const fn start() -> Self {
        Self {
            line: LineIndex(0),
            col: TextLengthUtf16(0),
        }
    }

    pub fn end(content: &str) -> Self {
        let mut line = 0;
        let mut col = 0;

        for c in content.chars() {
            if c == '\n' {
                line += 1;
                col = 0;
            } else {
                col += c.len_utf16();
            }
        }

        Self {
            line: LineIndex(line),
            col: TextLengthUtf16(col),
        }
    }
}

impl Ord for TextPointUtf16 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.line
            .cmp(&other.line)
            .then_with(|| self.col.cmp(&other.col))
    }
}

impl PartialOrd for TextPointUtf16 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<lsp_types::Position> for TextPointUtf16 {
    fn from(position: lsp_types::Position) -> Self {
        Self {
            line: LineIndex(position.line as usize),
            col: TextLengthUtf16(position.character as usize),
        }
    }
}

impl Add<TextPointDiffUtf16> for TextPointUtf16 {
    type Output = TextPointUtf16;

    fn add(self, area: TextPointDiffUtf16) -> Self::Output {
        let end = TextPointDiffUtf16::from(self) + area;

        Self {
            line: end.line,
            col: end.col,
        }
    }
}

impl AddAssign<TextPointDiffUtf16> for TextPointUtf16 {
    fn add_assign(&mut self, rhs: TextPointDiffUtf16) {
        *self = *self + rhs;
    }
}
