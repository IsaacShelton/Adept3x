mod from_float;
mod from_float_literal;
mod from_integer;
mod from_integer_literal;
mod from_pointer;
mod mode;

pub use self::mode::ConformMode;
use self::{
    from_float::from_float, from_float_literal::from_float_literal, from_integer::from_integer,
    from_integer_literal::from_integer_literal, from_pointer::from_pointer,
};
use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    ast::{ConformBehavior, FloatSize},
    resolved::{Expr, ExprKind, Type, TypeKind, TypedExpr},
    source_files::Source,
};
use num_bigint::BigInt;

pub fn conform_expr_or_error(
    expr: &TypedExpr,
    target_type: &Type,
    mode: ConformMode,
    behavior: ConformBehavior,
    conform_source: Source,
) -> Result<TypedExpr, ResolveError> {
    conform_expr(expr, target_type, mode, behavior, conform_source).ok_or_else(|| {
        ResolveErrorKind::TypeMismatch {
            left: expr.resolved_type.to_string(),
            right: target_type.to_string(),
        }
        .at(expr.expr.source)
    })
}

pub fn conform_expr(
    expr: &TypedExpr,
    to_type: &Type,
    mode: ConformMode,
    conform_behavior: ConformBehavior,
    conform_source: Source,
) -> Option<TypedExpr> {
    if expr.resolved_type == *to_type {
        return Some(expr.clone());
    }

    match &expr.resolved_type.kind {
        TypeKind::IntegerLiteral(from) => from_integer_literal(from, expr.expr.source, to_type),
        TypeKind::Integer(from_bits, from_sign) => from_integer(
            expr,
            mode,
            conform_behavior,
            *from_bits,
            *from_sign,
            to_type,
        ),
        TypeKind::FloatLiteral(from) => from_float_literal(*from, to_type, conform_source),
        TypeKind::Floating(from_size) => from_float(expr, *from_size, to_type),
        TypeKind::Pointer(from_inner) => from_pointer(expr, mode, from_inner, to_type),
        _ => None,
    }
}

pub fn conform_expr_to_default(expr: TypedExpr) -> Result<TypedExpr, ResolveError> {
    match &expr.resolved_type.kind {
        TypeKind::IntegerLiteral(value) => {
            conform_integer_literal_to_default_or_error(value, expr.expr.source)
        }
        TypeKind::FloatLiteral(value) => {
            Ok(conform_float_literal_to_default(*value, expr.expr.source))
        }
        _ => Ok(expr),
    }
}

fn conform_float_literal_to_default(value: f64, source: Source) -> TypedExpr {
    TypedExpr::new(
        TypeKind::f64().at(source),
        Expr::new(ExprKind::FloatingLiteral(FloatSize::Bits64, value), source),
    )
}

pub fn conform_integer_literal_to_default_or_error(
    value: &BigInt,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    integer_literal_to_default(value, source).ok_or_else(|| {
        ResolveErrorKind::UnrepresentableInteger {
            value: value.to_string(),
        }
        .at(source)
    })
}

fn integer_literal_to_default(value: &BigInt, source: Source) -> Option<TypedExpr> {
    for possible_type in [
        TypeKind::i32().at(source),
        TypeKind::u32().at(source),
        TypeKind::i64().at(source),
        TypeKind::u64().at(source),
    ] {
        if let Some(fit) = from_integer_literal(value, source, &possible_type) {
            return Some(fit);
        }
    }

    None
}
