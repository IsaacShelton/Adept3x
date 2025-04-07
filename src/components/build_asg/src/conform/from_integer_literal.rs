use super::{Objective, ObjectiveResult};
use asg::{Expr, ExprKind, Type, TypeKind, TypedExpr};
use ast::IntegerKnown;
use data_units::BitUnits;
use num::{BigInt, Zero};
use num_traits::ToPrimitive;
use ordered_float::NotNan;
use primitives::{
    CInteger, CIntegerAssumptions, FloatSize, IntegerBits, IntegerRigidity, IntegerSign,
};
use source_files::Source;

pub fn from_integer_literal<O: Objective>(
    value: &BigInt,
    assumptions: CIntegerAssumptions,
    source: Source,
    to_type: &Type,
) -> ObjectiveResult<O> {
    match &to_type.kind {
        TypeKind::Floating(to_size) => from_integer_literal_to_float::<O>(value, *to_size, source),
        TypeKind::CInteger(to_c_integer, to_sign) => from_integer_literal_to_c_integer::<O>(
            value,
            *to_c_integer,
            *to_sign,
            assumptions,
            source,
        ),
        TypeKind::Integer(to_bits, to_sign) => {
            from_integer_literal_to_integer::<O>(value, *to_bits, *to_sign, source)
        }
        TypeKind::SizeInteger(to_sign) => {
            from_integer_literal_to_size_integer::<O>(value, *to_sign, source)
        }
        _ => O::fail(),
    }
}

fn from_integer_literal_to_integer<O: Objective>(
    value: &BigInt,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
    source: Source,
) -> ObjectiveResult<O> {
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

    if does_fit {
        return O::success(|| {
            TypedExpr::new(
                TypeKind::Integer(to_bits, to_sign).at(source),
                ExprKind::IntegerKnown(Box::new(IntegerKnown {
                    rigidity: IntegerRigidity::Fixed(to_bits, to_sign),
                    value: value.clone(),
                }))
                .at(source),
            )
        });
    }

    O::fail()
}

fn from_integer_literal_to_c_integer<O: Objective>(
    value: &BigInt,
    to_c_integer: CInteger,
    to_sign: Option<IntegerSign>,
    assumptions: CIntegerAssumptions,
    source: Source,
) -> ObjectiveResult<O> {
    let needs_bits =
        BitUnits::of(value.bits() + (*value < BigInt::zero()).then_some(1).unwrap_or(0));

    if needs_bits <= to_c_integer.min_bits(assumptions).bits() {
        return O::success(|| {
            TypedExpr::new(
                TypeKind::CInteger(to_c_integer, to_sign).at(source),
                ExprKind::IntegerKnown(Box::new(IntegerKnown {
                    rigidity: IntegerRigidity::Loose(to_c_integer, to_sign),
                    value: value.clone(),
                }))
                .at(source),
            )
        });
    }

    O::fail()
}

fn from_integer_literal_to_size_integer<O: Objective>(
    value: &BigInt,
    to_sign: IntegerSign,
    source: Source,
) -> ObjectiveResult<O> {
    // Size types (i.e. size_t, ssize_t, usize, isize) are guananteed to be at least 16 bits
    // Anything more than that will require explicit casts
    let does_fit = match to_sign {
        IntegerSign::Signed => i16::try_from(value).is_ok(),
        IntegerSign::Unsigned => u16::try_from(value).is_ok(),
    };

    if does_fit {
        return O::success(|| {
            TypedExpr::new(
                TypeKind::SizeInteger(to_sign).at(source),
                ExprKind::IntegerKnown(Box::new(IntegerKnown {
                    rigidity: IntegerRigidity::Size(to_sign),
                    value: value.clone(),
                }))
                .at(source),
            )
        });
    }

    O::fail()
}

fn from_integer_literal_to_float<O: Objective>(
    value: &BigInt,
    to_size: FloatSize,
    source: Source,
) -> ObjectiveResult<O> {
    match value.to_f64() {
        Some(literal) => O::success(|| {
            TypedExpr::new(
                TypeKind::Floating(to_size).at(source),
                Expr::new(
                    ExprKind::FloatingLiteral(to_size, NotNan::new(literal).ok()),
                    source,
                ),
            )
        }),
        None => O::fail(),
    }
}
