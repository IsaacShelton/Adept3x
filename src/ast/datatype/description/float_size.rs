#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum FloatSize {
    Normal,
    Bits32,
    Bits64,
}

impl FloatSize {
    pub fn bits(self) -> u8 {
        match self {
            Self::Bits32 => 32,
            Self::Bits64 | Self::Normal => 64,
        }
    }
}
