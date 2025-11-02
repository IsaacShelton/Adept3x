use crate::{IntegerBits, IntegerSign};
use derive_more::From;
use num::BigInt;

#[derive(Copy, Clone, Debug, From)]
pub enum IntegerConstant {
    Signed(i64),
    Unsigned(u64),
}

impl IntegerConstant {
    pub fn new(value: BigInt) -> Option<Self> {
        if let Ok(unsigned) = u64::try_from(&value) {
            Some(Self::Unsigned(unsigned))
        } else if let Ok(signed) = i64::try_from(&value) {
            Some(Self::Signed(signed))
        } else {
            None
        }
    }

    pub fn from_le(raw_le_bytes: &[u8], sign: IntegerSign) -> Self {
        let le_bytes =
            std::array::from_fn::<u8, 8, _>(|i| raw_le_bytes.get(i).copied().unwrap_or(0));

        match sign {
            IntegerSign::Signed => Self::Signed(i64::from_le_bytes(le_bytes)),
            IntegerSign::Unsigned => Self::Unsigned(u64::from_le_bytes(le_bytes)),
        }
    }

    pub fn sign(&self) -> IntegerSign {
        match self {
            Self::Signed(_) => IntegerSign::Signed,
            Self::Unsigned(_) => IntegerSign::Unsigned,
        }
    }

    pub fn fits_in(&self, bits: IntegerBits) -> bool {
        match self {
            IntegerConstant::Signed(value) => {
                bits.min_signed() <= *value && *value <= bits.max_signed()
            }
            IntegerConstant::Unsigned(value) => {
                bits.min_unsigned() <= *value && *value <= bits.max_unsigned()
            }
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            IntegerConstant::Signed(value) => *value == 0,
            IntegerConstant::Unsigned(value) => *value == 0,
        }
    }

    pub fn unwrap_signed(&self) -> i64 {
        match self {
            IntegerConstant::Signed(value) => *value,
            IntegerConstant::Unsigned(_) => panic!("IntegerConstant::unwrap_signed"),
        }
    }

    pub fn unwrap_unsigned(&self) -> u64 {
        match self {
            IntegerConstant::Unsigned(value) => *value,
            IntegerConstant::Signed(_) => panic!("IntegerConstant::unwrap_unsigned"),
        }
    }

    pub fn raw_data(&self) -> u64 {
        match self {
            IntegerConstant::Signed(value) => *value as u64,
            IntegerConstant::Unsigned(value) => *value,
        }
    }
}

impl TryFrom<IntegerConstant> for u64 {
    type Error = ();

    fn try_from(value: IntegerConstant) -> Result<Self, Self::Error> {
        match value {
            IntegerConstant::Signed(value) => value.try_into().map_err(|_| ()),
            IntegerConstant::Unsigned(value) => value.try_into().map_err(|_| ()),
        }
    }
}
