use super::ConformMode;
use crate::{
    ast::{CInteger, IntegerBits, OptionIntegerSignExt},
    logic::implies,
    resolved::{
        Cast, CastFrom, ExprKind, IntegerSign, Type, TypeKind, TypedExpr, UnaryMathOperation,
        UnaryMathOperator,
    },
    source_files::Source,
};

pub fn from_c_integer(
    expr: &TypedExpr,
    mode: ConformMode,
    from_c_integer: CInteger,
    from_sign: Option<IntegerSign>,
    to_type: &Type,
    source: Source,
) -> Option<TypedExpr> {
    match &to_type.kind {
        TypeKind::Boolean => from_c_integer_to_bool(expr, mode, source),
        TypeKind::Integer(to_bits, to_sign) => from_c_integer_to_integer(
            expr,
            mode,
            from_c_integer,
            from_sign,
            *to_bits,
            *to_sign,
            source,
        ),
        TypeKind::CInteger(to_c_integer, to_sign) => from_c_integer_to_c_integer(
            expr,
            mode,
            from_c_integer,
            from_sign,
            *to_c_integer,
            *to_sign,
            source,
        ),
        _ => None,
    }
}

fn from_c_integer_to_bool(
    expr: &TypedExpr,
    mode: ConformMode,
    source: Source,
) -> Option<TypedExpr> {
    if !mode.allow_lossy_integer() {
        return None;
    }

    Some(TypedExpr::new(
        TypeKind::Boolean.at(source),
        ExprKind::UnaryMathOperation(Box::new(UnaryMathOperation {
            operator: UnaryMathOperator::IsNonZero,
            inner: expr.clone(),
        }))
        .at(source),
    ))
}

pub fn from_c_integer_to_c_integer(
    expr: &TypedExpr,
    mode: ConformMode,
    from_c_integer: CInteger,
    from_sign: Option<IntegerSign>,
    to_c_integer: CInteger,
    to_sign: Option<IntegerSign>,
    source: Source,
) -> Option<TypedExpr> {
    let target_type = TypeKind::CInteger(to_c_integer, to_sign).at(source);

    let is_smaller_likeness = from_sign == to_sign && from_c_integer <= to_c_integer;

    let is_smaller_and_can_preserve_sign =
        implies!(!from_sign.is_unsigned(), to_sign.is_signed()) && from_c_integer < to_c_integer;

    let is_lossless = is_smaller_likeness || is_smaller_and_can_preserve_sign;

    if mode.allow_lossy_integer() || is_lossless {
        return Some(TypedExpr::new(
            target_type.clone(),
            ExprKind::IntegerCast(Box::new(CastFrom {
                cast: Cast::new(target_type, expr.expr.clone()),
                from_type: TypeKind::CInteger(from_c_integer, from_sign).at(source),
            }))
            .at(source),
        ));
    }

    None
}

fn from_c_integer_to_integer(
    expr: &TypedExpr,
    mode: ConformMode,
    from_c_integer: CInteger,
    from_sign: Option<IntegerSign>,
    to_bits: IntegerBits,
    to_sign: IntegerSign,
    source: Source,
) -> Option<TypedExpr> {
    if !mode.allow_lossy_integer() {
        return None;
    }

    let target_type = TypeKind::Integer(to_bits, to_sign).at(source);

    Some(TypedExpr::new(
        target_type.clone(),
        ExprKind::IntegerCast(Box::new(CastFrom {
            cast: Cast::new(target_type, expr.expr.clone()),
            from_type: TypeKind::CInteger(from_c_integer, from_sign).at(source),
        }))
        .at(source),
    ))
}
