use data_units::{BitUnits, ByteUnits};

mod itanium;
mod record_info;

pub use itanium::*;
pub use record_info::*;

#[derive(Debug)]
pub struct ASTRecordLayout {
    pub size: ByteUnits,
    pub alignment: ByteUnits,
    pub preferred_alignment: ByteUnits,
    pub unadjusted_alignment: ByteUnits,
    pub required_alignment: ByteUnits,
    /// Size without tail padding
    pub data_size: ByteUnits,
    pub field_offsets: Vec<BitUnits>,
}
