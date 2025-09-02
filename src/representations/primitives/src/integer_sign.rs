use derive_more::IsVariant;
use num::{BigInt, Zero};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, IsVariant)]
pub enum IntegerSign {
    Signed,
    Unsigned,
}

impl IntegerSign {
    pub fn new(is_signed: bool) -> Self {
        if is_signed {
            Self::Signed
        } else {
            Self::Unsigned
        }
    }

    pub fn stronger(a: Self, b: Self) -> Self {
        (a.is_signed() || b.is_signed())
            .then_some(Self::Signed)
            .unwrap_or(Self::Unsigned)
    }

    pub fn strongest(a: Option<Self>, b: Option<Self>) -> Option<Self> {
        if a.is_signed() || b.is_signed() {
            Some(Self::Signed)
        } else if a.is_unsigned() || b.is_unsigned() {
            Some(Self::Unsigned)
        } else {
            None
        }
    }
}

impl From<&BigInt> for IntegerSign {
    fn from(value: &BigInt) -> Self {
        if *value < BigInt::zero() {
            Self::Signed
        } else {
            Self::Unsigned
        }
    }
}

pub trait OptionIntegerSignExt {
    fn is_signed(&self) -> bool;
    fn is_unsigned(&self) -> bool;
}

impl OptionIntegerSignExt for Option<IntegerSign> {
    fn is_signed(&self) -> bool {
        self.map_or(false, |sign| sign.is_signed())
    }

    fn is_unsigned(&self) -> bool {
        self.map_or(false, |sign| sign.is_unsigned())
    }
}
