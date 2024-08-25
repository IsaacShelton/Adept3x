use super::description::{IntegerBits, IntegerSign};
use crate::{data_units::ByteUnits, target::Target};
use derive_more::IsVariant;

#[derive(Copy, Clone, Debug, PartialEq, Eq, IsVariant, PartialOrd, Ord)]
pub enum CInteger {
    Char,
    Short,
    Int,
    Long,
    LongLong,
}

impl CInteger {
    pub fn largest(a: Option<Self>, b: Option<Self>) -> Option<Self> {
        if let (Some(a), Some(b)) = (a, b) {
            Some(a.max(b))
        } else {
            a.or(b)
        }
    }

    pub fn min_bits(self) -> IntegerBits {
        match self {
            Self::Char => IntegerBits::Bits8,
            Self::Short | Self::Int => IntegerBits::Bits16,
            Self::Long => IntegerBits::Bits32,
            Self::LongLong => IntegerBits::Bits64,
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Char => Some(Self::Short),
            Self::Short => Some(Self::Int),
            Self::Int => Some(Self::Long),
            Self::Long => Some(Self::LongLong),
            Self::LongLong => None,
        }
    }

    pub fn smallest_that_fits(min_c_integer: Self, bits: IntegerBits) -> Option<Self> {
        let mut possible = min_c_integer;

        loop {
            if possible.min_bits() >= bits {
                return Some(possible);
            }

            possible = if let Some(bigger) = possible.next() {
                bigger
            } else {
                return None;
            }
        }
    }

    pub fn bytes(&self, target: &Target) -> ByteUnits {
        match self {
            CInteger::Char => target.char_layout().width,
            CInteger::Short => target.short_layout().width,
            CInteger::Int => target.int_layout().width,
            CInteger::Long => target.long_layout().width,
            CInteger::LongLong => target.longlong_layout().width,
        }
    }
}

pub fn fmt_c_integer(
    f: &mut std::fmt::Formatter<'_>,
    integer: CInteger,
    sign: Option<IntegerSign>,
) -> std::fmt::Result {
    match sign {
        Some(IntegerSign::Signed) => {
            if integer.is_char() {
                f.write_str("signed ")?
            }
        }
        Some(IntegerSign::Unsigned) => f.write_str("unsigned ")?,
        None => (),
    }

    f.write_str(match integer {
        CInteger::Char => "char",
        CInteger::Short => "short",
        CInteger::Int => "int",
        CInteger::Long => "long",
        CInteger::LongLong => "long long",
    })?;

    Ok(())
}
