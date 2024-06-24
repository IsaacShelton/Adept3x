
pub type ByteCount = u32;

#[derive(Copy, Clone, Debug, Default)]
pub struct OffsetAlign {
    pub offset: ByteCount,
    pub align: ByteCount,
}

