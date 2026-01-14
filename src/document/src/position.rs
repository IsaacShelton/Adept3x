use text_edit::LineIndex;
use util_data_unit::ByteUnits;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DocumentPosition {
    pub line: LineIndex,
    pub index: ByteUnits,
}
