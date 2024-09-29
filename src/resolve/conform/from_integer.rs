use super::{ConformMode, Objective, ObjectiveResult};
use crate::{
    ast::{CInteger, ConformBehavior, IntegerBits},
    ir::IntegerSign,
    resolved::{Cast, CastFrom, Expr, ExprKind, Type, TypeKind, TypedExpr},
    source_files::Source,
};

pub fn from_integer<O: Objective>(
    expr: &TypedExpr,
    mode: ConformMode,
    behavior: ConformBehavior,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_type: &Type,
) -> ObjectiveResult<O> {
    match &to_type.kind {
        TypeKind::Integer(to_bits, to_sign) => match behavior {
            ConformBehavior::Adept(_) => from_integer_adept_mode::<O>(
                &expr.expr,
                mode,
                from_bits,
                from_sign,
                *to_bits,
                *to_sign,
                to_type.source,
            ),
            ConformBehavior::C => from_integer_c_mode::<O>(
                &expr.expr,
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
        let cast = Cast::new(target_type.clone(), expr.clone());

        let kind = if from_bits < to_bits {
            ExprKind::IntegerExtend(Box::new(cast))
        } else {
            ExprKind::IntegerTruncate(Box::new(cast))
        };

        return O::success(|| {
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
    expr: &TypedExpr,
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
        let cast_from = CastFrom {
            cast: Cast::new(target_type.clone(), expr.expr.clone()),
            from_type: expr.resolved_type.clone(),
        };

        return O::success(|| {
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
        TypedExpr::new(
            target_type.clone(),
            ExprKind::IntegerExtend(Box::new(Cast::new(target_type, expr.clone()))).at(expr.source),
        )
    })
}
