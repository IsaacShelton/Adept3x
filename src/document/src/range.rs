use crate::DocumentPosition;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DocumentRange {
    pub start: DocumentPosition,
    pub end: DocumentPosition,
}
