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
