pub mod itanium;

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct FieldOffset(u32);

#[derive(Clone, Debug)]
pub struct RecordLayout {
    pub size_bytes: u64,
    pub align_bytes: u64,
    pub preferred_align_bytes: u64,
    pub required_align_bytes: u64,

    /// Size without tail padding
    pub data_size_bytes: u64,

    pub field_offsets: Vec<FieldOffset>,
}
