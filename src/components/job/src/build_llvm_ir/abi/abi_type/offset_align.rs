use data_units::ByteUnits;

#[derive(Copy, Clone, Debug, Default)]
pub struct OffsetAlign {
    pub offset: ByteUnits,
    pub align: ByteUnits,
}
