use crate::{
    BuiltinTypes, ExecutionCtx, OnPolymorph, are_types_equal,
    conform::{
        Conform, UnaryCast, does_bit_integer_fit_in_c, does_integer_literal_fit,
        does_integer_literal_fit_in_c,
    },
    repr::{TypeKind, UnaliasedType},
    target_layout::TargetLayout,
};
use data_units::implies;
use derive_more::IsVariant;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use ordered_float::NotNan;
use primitives::{CIntegerAssumptions, IntegerBits, IntegerSign};
use source_files::Source;
use target::Target;

#[derive(Copy, Clone, Debug, Default, IsVariant)]
pub enum ConformMode {
    #[default]
    Normal,
    ParameterPassing,
    Explicit,
}

pub fn conform_to<'env>(
    ctx: &mut ExecutionCtx<'env>,
    original_from_ty: UnaliasedType<'env>,
    to_ty: UnaliasedType<'env>,
    assumptions: CIntegerAssumptions,
    builtin_types: &'env BuiltinTypes<'env>,
    target: &Target,
    _mode: ConformMode,
    on_polymorph: impl OnPolymorph<'env>,
    _source: Source,
) -> Option<Conform<'env>> {
    let mut from_ty = original_from_ty;
    let mut dereferences = 0;

    while !to_ty.0.kind.is_deref() {
        match &from_ty.0.kind {
            TypeKind::Deref(inner_type) => {
                from_ty = UnaliasedType(inner_type);
                dereferences += 1;
            }
            _ => break,
        }
    }

    if are_types_equal(from_ty, to_ty, on_polymorph) {
        return Some(Conform::identity(to_ty).after_implicit_dereferences(
            ctx,
            original_from_ty,
            dereferences,
        ));
    }

    let inner_conform = match &from_ty.0.kind {
        TypeKind::BooleanLiteral(value) => Some(Conform::new(
            builtin_types.bool(),
            UnaryCast::SpecializeBoolean(*value),
        )),
        TypeKind::IntegerLiteral(from) => match &to_ty.0.kind {
            TypeKind::IntegerLiteralInRange(min, max) => {
                if from >= min && from <= max {
                    Some(Conform::new(to_ty, UnaryCast::SpecializeInteger(from)))
                } else {
                    None
                }
            }
            TypeKind::FloatLiteral(to) => {
                if let Ok(true) = i64::try_from(*from)
                    .map(|x| x as f64)
                    .or_else(|_| u64::try_from(*from).map(|x| x as f64))
                    .or_else(|_| from.to_string().parse::<f64>())
                    .map(|float| /* Dubious comparison */ NotNan::<f64>::new(float).ok() == *to)
                {
                    Some(Conform::identity(to_ty))
                } else {
                    None
                }
            }
            TypeKind::BitInteger(to_bits, to_sign) => {
                does_integer_literal_fit(from, *to_bits, *to_sign)
                    .then(|| Conform::new(to_ty, UnaryCast::SpecializeInteger(from).into()))
            }
            TypeKind::CInteger(to_c_integer, to_sign) => {
                does_integer_literal_fit_in_c(from, *to_c_integer, *to_sign, assumptions, target)
                    .then(|| Conform::new(to_ty, UnaryCast::SpecializeInteger(from)))
            }
            TypeKind::SizeInteger(sign) => {
                let bits = IntegerBits::new(target.size_layout().width.to_bits())
                    .expect("size type to be representable with common bit integers");

                does_integer_literal_fit(from, bits, *sign)
                    .then(|| Conform::new(to_ty, UnaryCast::SpecializeInteger(from)))
            }
            TypeKind::Floating(_) => {
                // NOTE: to_f64 should be infallible despite signature, as overridden by BigInt
                from.to_f64().and_then(|from| {
                    Some(Conform::new(
                        to_ty,
                        UnaryCast::SpecializeFloat(NotNan::new(from).ok()),
                    ))
                })
            }
            _ => None,
        },
        TypeKind::IntegerLiteralInRange(from_min, from_max) => match &to_ty.0.kind {
            TypeKind::BitInteger(bits, sign) => {
                let (to_min, to_max) = if sign.is_signed() {
                    (
                        BigInt::from(bits.min_signed()),
                        BigInt::from(bits.max_signed()),
                    )
                } else {
                    (
                        BigInt::from(bits.min_unsigned()),
                        BigInt::from(bits.max_unsigned()),
                    )
                };

                if **from_min >= to_min && **from_max <= to_max {
                    let sign = IntegerSign::new(to_min < BigInt::ZERO);
                    Some(Conform::new(to_ty, UnaryCast::Extend(sign)))
                } else {
                    None
                }
            }
            TypeKind::CInteger(c_integer, sign) => {
                let sign = sign.unwrap_or_else(|| target.default_c_integer_sign(*c_integer));
                let bytes = target.c_integer_bytes(*c_integer);

                let range = if sign.is_signed() {
                    bytes
                        .min_max_signed()
                        .map(|(min, max)| (BigInt::from(min), BigInt::from(max)))
                } else {
                    bytes
                        .min_max_unsigned()
                        .map(|(min, max)| (BigInt::from(min), BigInt::from(max)))
                };

                if let Some((to_min, to_max)) = range {
                    if **from_min >= to_min && **from_max <= to_max {
                        let sign = IntegerSign::new(to_min < BigInt::ZERO);
                        Some(Conform::new(to_ty, UnaryCast::Extend(sign)))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            TypeKind::IntegerLiteralInRange(to_min, to_max) => {
                if from_min >= to_min && from_max <= to_max {
                    let sign = IntegerSign::new(**to_min < BigInt::ZERO);
                    Some(Conform::new(to_ty, UnaryCast::Extend(sign)))
                } else {
                    None
                }
            }
            _ => None,
        },
        TypeKind::BitInteger(from_bits, from_sign) => match &to_ty.0.kind {
            TypeKind::BitInteger(to_bits, to_sign) => (from_bits <= to_bits
                && implies!(from_sign.is_signed(), to_sign.is_signed()))
            .then(|| Conform::new(to_ty, UnaryCast::Extend(*from_sign))),
            TypeKind::CInteger(to_c_integer, to_sign) => does_bit_integer_fit_in_c(
                *from_bits,
                *from_sign,
                *to_c_integer,
                *to_sign,
                assumptions,
                target,
            )
            .then(|| Conform::new(to_ty, UnaryCast::Extend(*from_sign))),
            TypeKind::SizeInteger(to_sign) => {
                let to_bits = IntegerBits::new(target.size_layout().width.to_bits())
                    .expect("size type to be representable with common bit integers");

                (*from_bits <= to_bits && implies!(from_sign.is_signed(), to_sign.is_signed()))
                    .then(|| Conform::new(to_ty, UnaryCast::Extend(*from_sign)))
            }
            TypeKind::Floating(float_size) => todo!(),
            _ => None,
        },
        TypeKind::FloatLiteral(from) => todo!(),
        TypeKind::Floating(from_size) => todo!(),
        TypeKind::Ptr(_) => None,
        TypeKind::CInteger(from_size, from_sign) => todo!(),
        TypeKind::SizeInteger(from_sign) => todo!(),
        _ => None,
    }?;

    Some(inner_conform.after_implicit_dereferences(ctx, original_from_ty, dereferences))
}

/*
pub fn conform_expr<O: Objective>(
    ctx: &ResolveExprCtx,
    expr: &TypedExpr,
    to_type: &Type,
    mode: ConformMode,
    behavior: ConformBehavior,
    conform_source: Source,
) -> ObjectiveResult<O> {
    let Ok(from_type) = unalias(ctx.asg, &expr.ty) else {
        warn_type_alias_depth_exceeded(&expr.ty);
        return O::fail();
    };

    let Ok(to_type) = unalias(ctx.asg, to_type) else {
        warn_type_alias_depth_exceeded(to_type);
        return O::fail();
    };

    if *from_type == *to_type {
        return O::success(|| TypedExpr {
            ty: to_type.into_owned(),
            expr: expr.expr.clone(),
        });
    }

    match &from_type.kind {
        TypeKind::IntegerLiteral(from) => from_integer_literal::<O>(
            from,
            behavior.c_integer_assumptions(),
            expr.expr.source,
            &to_type,
        ),
        TypeKind::Integer(from_bits, from_sign) => from_integer::<O>(
            &expr.expr, &from_type, mode, behavior, *from_bits, *from_sign, &to_type,
        ),
        TypeKind::FloatLiteral(from) => from_float_literal::<O>(*from, &to_type, conform_source),
        TypeKind::Floating(from_size) => from_float::<O>(&expr.expr, mode, *from_size, &to_type),
        TypeKind::Ptr(from_inner) => from_pointer::<O>(ctx, &expr.expr, mode, from_inner, &to_type),
        TypeKind::CInteger(from_size, from_sign) => from_c_integer::<O>(
            &expr.expr,
            &from_type,
            mode,
            behavior,
            *from_size,
            *from_sign,
            &to_type,
            conform_source,
        ),
        TypeKind::SizeInteger(from_sign) => from_size_integer::<O>(
            &expr.expr,
            &from_type,
            mode,
            behavior,
            *from_sign,
            &to_type,
            conform_source,
        ),
        TypeKind::AnonymousEnum(enumeration) => from_anonymous_enum::<O>(
            &expr.expr,
            &from_type,
            mode,
            &to_type,
            enumeration.as_ref(),
            conform_source,
        ),
        _ => O::fail(),
    }
}
*/
