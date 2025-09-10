mod cast;
mod to;
mod to_default;

pub use cast::*;
use num_bigint::BigInt;
use num_traits::Zero;
use primitives::{CInteger, CIntegerAssumptions, IntegerBits, IntegerSign};
use target::Target;
pub use to::*;
pub use to_default::*;

macro_rules! implies {
    ($x:expr, $y:expr) => {
        !($x) || ($y)
    };
}

pub fn does_integer_literal_fit(value: &BigInt, bits: IntegerBits, sign: IntegerSign) -> bool {
    match (bits, sign) {
        (IntegerBits::Bits8, IntegerSign::Signed) => i8::try_from(value).is_ok(),
        (IntegerBits::Bits8, IntegerSign::Unsigned) => u8::try_from(value).is_ok(),
        (IntegerBits::Bits16, IntegerSign::Signed) => i16::try_from(value).is_ok(),
        (IntegerBits::Bits16, IntegerSign::Unsigned) => u16::try_from(value).is_ok(),
        (IntegerBits::Bits32, IntegerSign::Signed) => i32::try_from(value).is_ok(),
        (IntegerBits::Bits32, IntegerSign::Unsigned) => u32::try_from(value).is_ok(),
        (IntegerBits::Bits64, IntegerSign::Signed) => i64::try_from(value).is_ok(),
        (IntegerBits::Bits64, IntegerSign::Unsigned) => u64::try_from(value).is_ok(),
    }
}

pub fn does_integer_literal_fit_in_c(
    value: &BigInt,
    to_c_integer: CInteger,
    to_sign: Option<IntegerSign>,
    assumptions: CIntegerAssumptions,
    target: &Target,
) -> bool {
    let from_is_signed = *value < BigInt::zero();

    if to_sign.is_none() && from_is_signed && !assumptions.allow_target_implicit_sign_converion {
        return false;
    }

    let to_is_maybe_signed = if assumptions.allow_target_implicit_sign_converion {
        to_sign
            .unwrap_or_else(|| target.default_c_integer_sign(to_c_integer))
            .is_signed()
    } else {
        matches!(to_sign, None | Some(IntegerSign::Signed))
    };

    let min_c_int_bits = value.bits()
        + (to_is_maybe_signed && !from_is_signed)
            .then_some(1)
            .unwrap_or(0);

    implies!(from_is_signed, to_is_maybe_signed)
        && min_c_int_bits <= to_c_integer.min_bits(assumptions).bits().bits()
}
