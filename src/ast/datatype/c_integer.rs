use super::description::{IntegerBits, IntegerSign};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CInteger {
    Char,
    Short,
    Int,
    Long,
    LongLong,
}

impl CInteger {
    pub fn min_bits(self) -> IntegerBits {
        match self {
            Self::Char => IntegerBits::Bits8,
            Self::Short | Self::Int => IntegerBits::Bits16,
            Self::Long => IntegerBits::Bits32,
            Self::LongLong => IntegerBits::Bits64,
        }
    }
}

pub fn fmt_c_integer(
    f: &mut std::fmt::Formatter<'_>,
    integer: CInteger,
    sign: Option<IntegerSign>,
) -> std::fmt::Result {
    match sign {
        Some(IntegerSign::Signed) => f.write_str("signed ")?,
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
