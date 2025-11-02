use data_units::{BitUnits, ByteUnits};
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
        self.bytes().to_bits()
    }

    pub fn bytes(self) -> ByteUnits {
        match self {
            IntegerBits::Bits8 => ByteUnits::of(1),
            IntegerBits::Bits16 => ByteUnits::of(2),
            IntegerBits::Bits32 => ByteUnits::of(4),
            IntegerBits::Bits64 => ByteUnits::of(8),
        }
    }

    pub fn min_signed(&self) -> i64 {
        match self {
            IntegerBits::Bits8 => i8::MIN.into(),
            IntegerBits::Bits16 => i16::MIN.into(),
            IntegerBits::Bits32 => i32::MIN.into(),
            IntegerBits::Bits64 => i64::MIN,
        }
    }

    pub fn max_signed(&self) -> i64 {
        match self {
            IntegerBits::Bits8 => i8::MAX.into(),
            IntegerBits::Bits16 => i16::MAX.into(),
            IntegerBits::Bits32 => i32::MAX.into(),
            IntegerBits::Bits64 => i64::MAX,
        }
    }

    pub fn min_unsigned(&self) -> u64 {
        match self {
            IntegerBits::Bits8 => u8::MIN.into(),
            IntegerBits::Bits16 => u16::MIN.into(),
            IntegerBits::Bits32 => u32::MIN.into(),
            IntegerBits::Bits64 => u64::MIN,
        }
    }

    pub fn max_unsigned(&self) -> u64 {
        match self {
            IntegerBits::Bits8 => u8::MAX.into(),
            IntegerBits::Bits16 => u16::MAX.into(),
            IntegerBits::Bits32 => u32::MAX.into(),
            IntegerBits::Bits64 => u64::MAX,
        }
    }

    pub fn mask(&self) -> u64 {
        self.max_unsigned()
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
