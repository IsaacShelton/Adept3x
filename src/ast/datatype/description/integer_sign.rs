use derive_more::IsVariant;
use num::{BigInt, Zero};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, IsVariant)]
pub enum IntegerSign {
    Signed,
    Unsigned,
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
