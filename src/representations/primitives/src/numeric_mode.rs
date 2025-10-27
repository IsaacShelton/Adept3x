use super::{CInteger, IntegerBits, IntegerSign};
use crate::FloatOrInteger;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum NumericMode {
    Integer(IntegerSign),
    LooseIndeterminateSignInteger(CInteger),
    CheckOverflow(IntegerBits, IntegerSign),
    Float,
}

impl NumericMode {
    pub fn float_or_integer(&self) -> FloatOrInteger {
        match self {
            Self::Integer(..)
            | Self::LooseIndeterminateSignInteger(_)
            | Self::CheckOverflow(_, _) => FloatOrInteger::Integer,
            Self::Float => FloatOrInteger::Float,
        }
    }
}
