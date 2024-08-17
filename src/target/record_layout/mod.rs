use crate::data_units::{BitUnits, ByteUnits};

pub mod itanium;
pub mod record_info;

#[derive(Clone, Debug)]
pub struct RecordLayout {
    pub size: ByteUnits,
    pub alignment: ByteUnits,
    pub preferred_alignment: ByteUnits,
    pub required_alignment: ByteUnits,

    /// Size without tail padding
    pub data_size: ByteUnits,

    pub field_offsets: Vec<BitUnits>,
}
