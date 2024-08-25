use crate::data_units::{BitUnits, ByteUnits};
use std::cmp::Ordering;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IntegerBits {
    Bits8,
    Bits16,
    Bits32,
    Bits64,
}

impl IntegerBits {
    pub fn new(bits: BitUnits) -> Option<Self> {
        if bits <= BitUnits::of(8) {
            Some(Self::Bits8)
        } else if bits <= BitUnits::of(16) {
            Some(Self::Bits16)
        } else if bits <= BitUnits::of(32) {
            Some(Self::Bits32)
        } else if bits <= BitUnits::of(64) {
            Some(Self::Bits64)
        } else {
            None
        }
    }

    pub fn successor(self) -> Option<IntegerBits> {
        match self {
            Self::Bits8 => Some(Self::Bits16),
            Self::Bits16 => Some(Self::Bits32),
            Self::Bits32 => Some(Self::Bits64),
            Self::Bits64 => None,
        }
    }

    pub fn bits(self) -> BitUnits {
        match self {
            IntegerBits::Bits8 => BitUnits::of(8),
            IntegerBits::Bits16 => BitUnits::of(16),
            IntegerBits::Bits32 => BitUnits::of(32),
            IntegerBits::Bits64 => BitUnits::of(64),
        }
    }

    pub fn bytes(self) -> ByteUnits {
        match self {
            IntegerBits::Bits8 => ByteUnits::of(1),
            IntegerBits::Bits16 => ByteUnits::of(2),
            IntegerBits::Bits32 => ByteUnits::of(4),
            IntegerBits::Bits64 => ByteUnits::of(8),
        }
    }
}

impl Ord for IntegerBits {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bits().cmp(&other.bits())
    }
}

impl PartialOrd for IntegerBits {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl TryFrom<ByteUnits> for IntegerBits {
    type Error = ();

    fn try_from(value: ByteUnits) -> Result<Self, Self::Error> {
        IntegerBits::new(value.to_bits()).ok_or(())
    }
}

impl TryFrom<BitUnits> for IntegerBits {
    type Error = ();

    fn try_from(value: BitUnits) -> Result<Self, Self::Error> {
        IntegerBits::new(value).ok_or(())
    }
}
