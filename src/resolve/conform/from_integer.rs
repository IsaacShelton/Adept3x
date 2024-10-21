use super::{ConformMode, Objective, ObjectiveResult};
use crate::{
    ast::{CInteger, ConformBehavior, IntegerBits},
    ir::IntegerSign,
    resolved::{Cast, CastFrom, Expr, ExprKind, Type, TypeKind, TypedExpr},
    source_files::Source,
};

pub fn from_integer<O: Objective>(
    expr: &Expr,
    from_type: &Type,
    mode: ConformMode,
    behavior: ConformBehavior,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_type: &Type,
) -> ObjectiveResult<O> {
    match &to_type.kind {
        TypeKind::Integer(to_bits, to_sign) => match behavior {
            ConformBehavior::Adept(_) => from_integer_adept_mode::<O>(
                &expr,
                from_type,
                mode,
                from_bits,
                from_sign,
                *to_bits,
                *to_sign,
                to_type.source,
            ),
            ConformBehavior::C => from_integer_c_mode::<O>(
                &expr,
                from_type,
                mode,
                from_bits,
                from_sign,
                *to_bits,
                *to_sign,
                to_type.source,
            ),
        },
        TypeKind::CInteger(to_c_integer, to_sign) => conform_from_integer_to_c_integer::<O>(
            expr,
            from_type,
            mode,
            behavior,
            from_bits,
            from_sign,
            *to_c_integer,
            *to_sign,
            to_type.source,
        ),
        _ => O::fail(),
    }
}

fn from_integer_c_mode<O: Objective>(
    expr: &Expr,
    from_type: &Type,
    mode: ConformMode,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
    type_source: Source,
) -> ObjectiveResult<O> {
    if !mode.allow_lossless_integer() {
        return O::fail();
    }

    let target_type = TypeKind::Integer(to_bits, to_sign).at(type_source);

    if from_bits == to_bits && from_sign == to_sign {
        return O::success(|| TypedExpr::new(target_type, expr.clone()));
    }

    if mode.allow_lossy_integer() {
        return O::success(|| {
            let cast = Cast::new(target_type.clone(), expr.clone());

            let kind = if from_bits < to_bits {
                ExprKind::IntegerExtend(Box::new(CastFrom {
                    cast,
                    from_type: from_type.clone(),
                }))
            } else {
                ExprKind::IntegerTruncate(Box::new(cast))
            };

            TypedExpr::new(
                target_type,
                Expr {
                    kind,
                    source: expr.source,
                },
            )
        });
    }

    todo!("conform_integer_value_c {:?}", mode);
}

fn conform_from_integer_to_c_integer<O: Objective>(
    expr: &Expr,
    from_type: &Type,
    mode: ConformMode,
    behavior: ConformBehavior,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_c_integer: CInteger,
    to_sign: Option<IntegerSign>,
    source: Source,
) -> ObjectiveResult<O> {
    if !mode.allow_lossless_integer() {
        return O::fail();
    }

    let target_type = TypeKind::CInteger(to_c_integer, to_sign).at(source);
    let assumptions = behavior.c_integer_assumptions();

    let is_lossless = match to_sign {
        Some(to_sign) if from_sign == to_sign => from_bits <= to_c_integer.min_bits(assumptions),
        Some(to_sign) if to_sign.is_signed() => from_bits < to_c_integer.min_bits(assumptions),
        _ => false,
    };

    if mode.allow_lossy_integer() || is_lossless {
        return O::success(|| {
            let cast_from = CastFrom {
                cast: Cast::new(target_type.clone(), expr.clone()),
                from_type: from_type.clone(),
            };

            TypedExpr::new(
                target_type,
                ExprKind::IntegerCast(Box::new(cast_from)).at(source),
            )
        });
    }

    O::fail()
}

fn from_integer_adept_mode<O: Objective>(
    expr: &Expr,
    from_type: &Type,
    mode: ConformMode,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
    type_source: Source,
) -> ObjectiveResult<O> {
    if !mode.allow_lossless_integer() {
        return O::fail();
    }

    if to_bits < from_bits || (from_sign != to_sign && to_bits == from_bits) {
        return O::fail();
    }

    let target_type = TypeKind::Integer(to_bits, to_sign).at(type_source);

    if to_sign == from_sign && to_bits == from_bits {
        return O::success(|| TypedExpr::new(target_type, expr.clone()));
    }

    O::success(|| {
        let cast = Cast::new(target_type.clone(), expr.clone());

        TypedExpr::new(
            target_type,
            ExprKind::IntegerExtend(Box::new(CastFrom {
                cast,
                from_type: from_type.clone(),
            }))
            .at(expr.source),
        )
    })
}
