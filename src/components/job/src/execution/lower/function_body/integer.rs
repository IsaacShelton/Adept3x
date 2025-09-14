use crate::ir::{self, Literal};
use data_units::BitUnits;
use diagnostics::ErrorDiagnostic;
use num_bigint::BigInt;
use primitives::{IntegerBits, IntegerSign};
use source_files::Source;

pub fn value_for_bit_integer(
    value: &BigInt,
    bits: IntegerBits,
    sign: IntegerSign,
    source: Source,
) -> Result<ir::Value, ErrorDiagnostic> {
    match (bits, sign) {
        (IntegerBits::Bits8, IntegerSign::Signed) => {
            value.try_into().map(Literal::Signed8).map_err(|_| "i8")
        }
        (IntegerBits::Bits8, IntegerSign::Unsigned) => {
            value.try_into().map(Literal::Unsigned8).map_err(|_| "u8")
        }
        (IntegerBits::Bits16, IntegerSign::Signed) => {
            value.try_into().map(Literal::Signed16).map_err(|_| "i16")
        }
        (IntegerBits::Bits16, IntegerSign::Unsigned) => {
            value.try_into().map(Literal::Unsigned16).map_err(|_| "u16")
        }
        (IntegerBits::Bits32, IntegerSign::Signed) => {
            value.try_into().map(Literal::Signed32).map_err(|_| "i32")
        }
        (IntegerBits::Bits32, IntegerSign::Unsigned) => {
            value.try_into().map(Literal::Unsigned32).map_err(|_| "u32")
        }
        (IntegerBits::Bits64, IntegerSign::Signed) => {
            value.try_into().map(Literal::Signed64).map_err(|_| "i64")
        }
        (IntegerBits::Bits64, IntegerSign::Unsigned) => {
            value.try_into().map(Literal::Unsigned64).map_err(|_| "u64")
        }
    }
    .map(|literal| ir::Value::Literal(literal))
    .map_err(|expected_type| {
        ErrorDiagnostic::new(
            format!("Cannot fit value {} in '{}'", value, expected_type),
            source,
        )
    })
}

pub fn bits_and_sign_for_invisible_integer(
    value: &BigInt,
) -> Result<(IntegerBits, IntegerSign), ()> {
    bits_and_sign_for_invisible_integer_in_range(value, value)
}

pub fn bits_and_sign_for_invisible_integer_in_range(
    min: &BigInt,
    max: &BigInt,
) -> Result<(IntegerBits, IntegerSign), ()> {
    let signed = *min < BigInt::ZERO || *max < BigInt::ZERO;
    let bits = IntegerBits::new(BitUnits::of(min.bits().max(max.bits()) + signed as u64));
    bits.map(|bits| (bits, IntegerSign::new(signed))).ok_or(())
}
