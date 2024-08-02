use super::IntegerBits;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IntegerFixedBits {
    Bits8,
    Bits16,
    Bits32,
    Bits64,
}

impl From<IntegerFixedBits> for IntegerBits {
    fn from(value: IntegerFixedBits) -> Self {
        match value {
            IntegerFixedBits::Bits8 => Self::Bits8,
            IntegerFixedBits::Bits16 => Self::Bits16,
            IntegerFixedBits::Bits32 => Self::Bits32,
            IntegerFixedBits::Bits64 => Self::Bits64,
        }
    }
}
