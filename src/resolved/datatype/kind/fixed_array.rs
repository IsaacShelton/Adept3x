use crate::resolved::Type;

#[derive(Clone, Debug, PartialEq)]
pub struct FixedArray {
    pub size: u64,
    pub inner: Type,
}
