use derive_more::{Add, AddAssign, Sub, SubAssign, Sum};
use serde::{Deserialize, Serialize};

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
pub struct TextLengthUtf16(pub usize);

impl TextLengthUtf16 {
    pub fn of_str(content: &str) -> Self {
        Self(content.chars().map(|c| c.len_utf16()).sum())
    }
}
