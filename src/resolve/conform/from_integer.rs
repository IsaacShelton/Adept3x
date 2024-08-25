use super::ConformMode;
use crate::{
    ast::{CInteger, ConformBehavior, IntegerBits},
    data_units::BitUnits,
    ir::IntegerSign,
    resolved::{Cast, CastFrom, Expr, ExprKind, Type, TypeKind, TypedExpr},
    source_files::Source,
};

pub fn from_integer(
    expr: &TypedExpr,
    mode: ConformMode,
    behavior: ConformBehavior,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_type: &Type,
) -> Option<TypedExpr> {
    match &to_type.kind {
        TypeKind::Integer(to_bits, to_sign) => match behavior {
            ConformBehavior::Adept => from_integer_adept_mode(
                &expr.expr,
                from_bits,
                from_sign,
                *to_bits,
                *to_sign,
                to_type.source,
            ),
            ConformBehavior::C => from_integer_c_mode(
                &expr.expr,
                mode,
                from_bits,
                from_sign,
                *to_bits,
                *to_sign,
                to_type.source,
            ),
        },
        TypeKind::CInteger(to_c_integer, to_sign) => conform_from_integer_to_c_integer(
            expr,
            mode,
            from_bits,
            from_sign,
            *to_c_integer,
            *to_sign,
            to_type.source,
        ),
        _ => None,
    }
}

fn from_integer_c_mode(
    expr: &Expr,
    conform_mode: ConformMode,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
    type_source: Source,
) -> Option<TypedExpr> {
    let target_type = TypeKind::Integer(to_bits, to_sign).at(type_source);

    if from_bits == to_bits && from_sign == to_sign {
        return Some(TypedExpr::new(target_type, expr.clone()));
    }

    if conform_mode.allow_lossy_integer() {
        let cast = Cast::new(target_type.clone(), expr.clone());

        let kind = if from_bits < to_bits {
            ExprKind::IntegerExtend(Box::new(cast))
        } else {
            ExprKind::IntegerTruncate(Box::new(cast))
        };

        return Some(TypedExpr::new(
            target_type,
            Expr {
                kind,
                source: expr.source,
            },
        ));
    }

    todo!("conform_integer_value_c {:?}", conform_mode);
}

fn conform_from_integer_to_c_integer(
    expr: &TypedExpr,
    conform_mode: ConformMode,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    c_integer: CInteger,
    sign: Option<IntegerSign>,
    source: Source,
) -> Option<TypedExpr> {
    let target_type = TypeKind::CInteger(c_integer, sign).at(source);

    let needed_bits = if from_sign.is_unsigned() {
        BitUnits::of(from_bits.bits().into()) + BitUnits::of(1)
    } else {
        BitUnits::of(from_bits.bits().into())
    };

    let is_lossless = match sign {
        Some(to_sign) if sign == Some(to_sign) && (to_sign.is_signed() || to_sign == from_sign) => {
            needed_bits <= BitUnits::of(c_integer.min_bits().bits().into())
        }
        _ => false,
    };

    if conform_mode.allow_lossy_integer() || is_lossless {
        let cast_from = CastFrom {
            cast: Cast::new(target_type.clone(), expr.expr.clone()),
            from_type: expr.resolved_type.clone(),
        };

        Some(TypedExpr::new(
            target_type,
            ExprKind::IntegerCast(Box::new(cast_from)).at(source),
        ))
    } else {
        None
    }
}

fn from_integer_adept_mode(
    expr: &Expr,
    from_bits: IntegerBits,
    from_sign: IntegerSign,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
    type_source: Source,
) -> Option<TypedExpr> {
    if to_bits < from_bits || (from_sign != to_sign && to_bits == from_bits) {
        return None;
    }

    let target_type = TypeKind::Integer(to_bits, to_sign).at(type_source);

    if to_sign == from_sign && to_bits == from_bits {
        return Some(TypedExpr::new(target_type, expr.clone()));
    }

    Some(TypedExpr::new(
        target_type.clone(),
        ExprKind::IntegerExtend(Box::new(Cast::new(target_type, expr.clone()))).at(expr.source),
    ))
}
