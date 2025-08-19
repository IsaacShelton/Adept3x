use crate::{
    BuiltinTypes, ExecutionCtx, Resolved, UnaryImplicitCast,
    repr::{TypeKind, UnaliasedType},
};
use data_units::BitUnits;
use diagnostics::ErrorDiagnostic;
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};
use ordered_float::NotNan;
use primitives::{CIntegerAssumptions, IntegerBits, IntegerSign};
use source_files::Source;

pub fn conform_to_default<'env>(
    ctx: &mut ExecutionCtx<'env>,
    ty: UnaliasedType<'env>,
    assumptions: CIntegerAssumptions,
    builtin_types: &'env BuiltinTypes<'env>,
) -> Result<Resolved<'env>, ErrorDiagnostic> {
    let source = ty.0.source;

    Ok(match &ty.0.kind {
        TypeKind::IntegerLiteral(big_int) => [
            &builtin_types.i32,
            &builtin_types.u32,
            &builtin_types.i64,
            &builtin_types.u64,
        ]
        .into_iter()
        .flat_map(|possible_type| {
            from_integer_literal(
                ctx,
                big_int,
                assumptions,
                source,
                &possible_type.kind,
                builtin_types,
            )
        })
        .next()
        .ok_or_else(|| ErrorDiagnostic::new("Failed to specialize integer literal", source))?,
        TypeKind::FloatLiteral(not_nan) => Resolved::new(
            builtin_types.f64(),
            UnaryImplicitCast::SpecializeFloat(not_nan.clone()).into(),
        ),
        TypeKind::BooleanLiteral(value) => Resolved::new(
            builtin_types.bool(),
            UnaryImplicitCast::SpecializeBoolean(*value).into(),
        ),
        _ => Resolved::from_type(ty.clone()),
    })
}

fn from_integer_literal<'env>(
    ctx: &mut ExecutionCtx<'env>,
    value: &BigInt,
    assumptions: CIntegerAssumptions,
    source: Source,
    to_type_kind: &'env TypeKind<'env>,
    builtin_types: &'env BuiltinTypes<'env>,
) -> Option<Resolved<'env>> {
    match &to_type_kind {
        TypeKind::Floating(float_size) => value.to_f64().map(|float| {
            Resolved::new(
                builtin_types.floating(*float_size),
                UnaryImplicitCast::SpecializeFloat(NotNan::new(float).ok()).into(),
            )
        }),
        TypeKind::BitInteger(to_bits, to_sign) => {
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
                Resolved::new(
                    UnaliasedType(ctx.alloc(TypeKind::BitInteger(*to_bits, *to_sign).at(source))),
                    UnaryImplicitCast::SpecializeInteger(value.clone()).into(),
                )
            })
        }
        TypeKind::CInteger(to_c_integer, to_sign) => {
            let needs_bits =
                BitUnits::of(value.bits() + (*value < BigInt::zero()).then_some(1).unwrap_or(0));

            (needs_bits <= to_c_integer.min_bits(assumptions).bits()).then(|| {
                Resolved::new(
                    UnaliasedType(
                        ctx.alloc(TypeKind::CInteger(*to_c_integer, *to_sign).at(source)),
                    ),
                    UnaryImplicitCast::SpecializeInteger(value.clone()).into(),
                )
            })
        }
        TypeKind::SizeInteger(to_sign) => {
            // Size types (i.e. size_t, ssize_t, usize, isize) are guananteed to be at least 16 bits
            // Anything more than that will require explicit casts
            let does_fit = match to_sign {
                IntegerSign::Signed => i16::try_from(value).is_ok(),
                IntegerSign::Unsigned => u16::try_from(value).is_ok(),
            };

            does_fit.then(|| {
                Resolved::new(
                    UnaliasedType(ctx.alloc(TypeKind::SizeInteger(*to_sign).at(source))),
                    UnaryImplicitCast::SpecializeInteger(value.clone()).into(),
                )
            })
        }
        _ => None,
    }
}
