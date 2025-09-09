use data_units::ByteUnits;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Hash)]
pub enum FloatSize {
    Bits32,
    Bits64,
}

impl FloatSize {
    pub fn bytes(self) -> ByteUnits {
        match self {
            Self::Bits32 => ByteUnits::of(4),
            Self::Bits64 => ByteUnits::of(8),
        }
    }
}
