use crate::resolved::Type;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FixedArray {
    pub size: u64,
    pub inner: Type,
}
