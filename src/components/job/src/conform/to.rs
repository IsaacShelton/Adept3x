use crate::{
    BuiltinTypes, ExecutionCtx, OnPolymorph, are_types_equal,
    conform::{Conform, UnaryCast, does_integer_literal_fit, does_integer_literal_fit_in_c},
    repr::{TypeKind, UnaliasedType},
};
use derive_more::IsVariant;
use ordered_float::NotNan;
use primitives::CIntegerAssumptions;
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
    target: &Target,
    _builtin_types: &'env BuiltinTypes<'env>,
    _mode: ConformMode,
    on_polymorph: impl OnPolymorph<'env>,
    source: Source,
) -> Option<Conform<'env>> {
    let (from_ty, needs_dereference) = match &original_from_ty.0.kind {
        TypeKind::Deref(inner_type, _) => (UnaliasedType(inner_type), true),
        _ => (original_from_ty, false),
    };

    if are_types_equal(from_ty, to_ty, on_polymorph) {
        let inner_conform = Conform::identity(to_ty);

        return Some(if needs_dereference {
            inner_conform.after_dereference(ctx)
        } else {
            inner_conform
        });
    }

    let inner_conform = match &from_ty.0.kind {
        TypeKind::IntegerLiteral(from) => match &to_ty.0.kind {
            TypeKind::IntegerLiteralInRange(min, max) => {
                if from >= min && from <= max {
                    Some(Conform::identity(to_ty))
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
                does_integer_literal_fit(from, *to_bits, *to_sign).then(|| {
                    Conform::new(
                        UnaliasedType(
                            ctx.alloc(TypeKind::BitInteger(*to_bits, *to_sign).at(source)),
                        ),
                        UnaryCast::SpecializeInteger(from).into(),
                    )
                })
            }
            TypeKind::CInteger(to_c_integer, to_sign) => {
                does_integer_literal_fit_in_c(from, *to_c_integer, *to_sign, assumptions, target)
                    .then(|| {
                        Conform::new(
                            UnaliasedType(
                                ctx.alloc(TypeKind::CInteger(*to_c_integer, *to_sign).at(source)),
                            ),
                            UnaryCast::SpecializeInteger(from),
                        )
                    })
            }
            TypeKind::SizeInteger(integer_sign) => todo!(),
            TypeKind::Floating(float_size) => todo!(),
            _ => None,
        },
        TypeKind::BitInteger(from_bits, from_sign) => todo!(),
        TypeKind::FloatLiteral(from) => todo!(),
        TypeKind::Floating(from_size) => todo!(),
        TypeKind::Ptr(_) => None,
        TypeKind::CInteger(from_size, from_sign) => todo!(),
        TypeKind::SizeInteger(from_sign) => todo!(),
        _ => None,
    }?;

    if needs_dereference {
        Some(inner_conform.after_dereference(ctx))
    } else {
        Some(inner_conform)
    }
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
