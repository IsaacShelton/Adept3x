use crate::ir::{self, IntegerImmediate};
use data_units::BitUnits;
use diagnostics::ErrorDiagnostic;
use num_bigint::BigInt;
use primitives::{IntegerBits, IntegerSign};
use source_files::Source;

pub fn literal_value_for_bit_integer(
    value: &BigInt,
    bits: IntegerBits,
    sign: IntegerSign,
    source: Source,
) -> Result<ir::Literal, ErrorDiagnostic> {
    IntegerImmediate::new_with_bits_and_sign(value, sign, bits)
        .map(ir::Literal::Integer)
        .ok_or_else(|| {
            ErrorDiagnostic::new(
                format!(
                    "Cannot fit value {} in `{}{}`",
                    value,
                    sign.prefix(),
                    bits.bytes().bytes()
                ),
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
