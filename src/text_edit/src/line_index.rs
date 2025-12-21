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
pub struct LineIndex(pub usize);
