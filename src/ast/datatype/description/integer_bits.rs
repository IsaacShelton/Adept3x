use std::cmp::Ordering;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IntegerBits {
    Bits8,
    Bits16,
    Bits32,
    Bits64,
}

impl IntegerBits {
    pub fn new(bits: u64) -> Option<Self> {
        if bits <= 8 {
            Some(Self::Bits8)
        } else if bits <= 16 {
            Some(Self::Bits16)
        } else if bits <= 32 {
            Some(Self::Bits32)
        } else if bits <= 64 {
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

    pub fn bits(self) -> u8 {
        match self {
            IntegerBits::Bits8 => 8,
            IntegerBits::Bits16 => 16,
            IntegerBits::Bits32 => 32,
            IntegerBits::Bits64 => 64,
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
