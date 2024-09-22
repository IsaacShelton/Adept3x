use crate::{
    ast::{CInteger, CIntegerAssumptions, FloatSize, IntegerBits, IntegerKnown, IntegerRigidity},
    data_units::BitUnits,
    ir::IntegerSign,
    resolved::{Expr, ExprKind, Type, TypeKind, TypedExpr},
    source_files::Source,
};
use num::{BigInt, Zero};
use num_traits::ToPrimitive;

pub fn from_integer_literal(
    value: &BigInt,
    assumptions: CIntegerAssumptions,
    source: Source,
    to_type: &Type,
) -> Option<TypedExpr> {
    match &to_type.kind {
        TypeKind::Floating(to_size) => from_integer_literal_to_float(value, *to_size, source),
        TypeKind::CInteger(to_c_integer, to_sign) => {
            from_integer_literal_to_c_integer(value, *to_c_integer, *to_sign, assumptions, source)
        }
        TypeKind::Integer(to_bits, to_sign) => {
            from_integer_literal_to_integer(value, *to_bits, *to_sign, source)
        }
        _ => None,
    }
}

fn from_integer_literal_to_integer(
    value: &BigInt,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
    source: Source,
) -> Option<TypedExpr> {
    let does_fit = match (to_bits, to_sign) {
        (IntegerBits::Bits8, IntegerSign::Signed) => i8::try_from(value).is_ok(),
        (IntegerBits::Bits8, IntegerSign::Unsigned) => u8::try_from(value).is_ok(),
        (IntegerBits::Bits16, IntegerSign::Signed) => i16::try_from(value).is_ok(),
        (IntegerBits::Bits16, IntegerSign::Unsigned) => u16::try_from(value).is_ok(),
        (IntegerBits::Bits32, IntegerSign::Signed) => i32::try_from(value).is_ok(),
        (IntegerBits::Bits32, IntegerSign::Unsigned) => u32::try_from(value).is_ok(),
        (IntegerBits::Bits64, IntegerSign::Signed) => i64::try_from(value).is_ok(),
        (IntegerBits::Bits64, IntegerSign::Unsigned) => u64::try_from(value).is_ok(),
    };

    does_fit.then(|| {
        TypedExpr::new(
            TypeKind::Integer(to_bits, to_sign).at(source),
            ExprKind::IntegerKnown(Box::new(IntegerKnown {
                rigidity: IntegerRigidity::Fixed(to_bits),
                value: value.clone(),
                sign: to_sign,
            }))
            .at(source),
        )
    })
}

fn from_integer_literal_to_c_integer(
    value: &BigInt,
    to_c_integer: CInteger,
    to_sign: Option<IntegerSign>,
    assumptions: CIntegerAssumptions,
    source: Source,
) -> Option<TypedExpr> {
    let needs_bits =
        BitUnits::of(value.bits() + (*value < BigInt::zero()).then_some(1).unwrap_or(0));

    (needs_bits <= to_c_integer.min_bits(assumptions).bits()).then(|| {
        TypedExpr::new(
            TypeKind::CInteger(to_c_integer, to_sign).at(source),
            ExprKind::IntegerKnown(Box::new(IntegerKnown {
                rigidity: IntegerRigidity::Loose(to_c_integer),
                value: value.clone(),
                sign: to_sign.unwrap_or(IntegerSign::Signed),
            }))
            .at(source),
        )
    })
}

fn from_integer_literal_to_float(
    value: &BigInt,
    to_size: FloatSize,
    source: Source,
) -> Option<TypedExpr> {
    value.to_f64().map(|literal| {
        TypedExpr::new(
            TypeKind::Floating(to_size).at(source),
            Expr::new(ExprKind::FloatingLiteral(to_size, literal), source),
        )
    })
}
