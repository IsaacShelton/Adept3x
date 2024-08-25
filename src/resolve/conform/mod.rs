mod from_c_integer;
mod from_float;
mod from_float_literal;
mod from_integer;
mod from_integer_literal;
mod from_pointer;
mod mode;
pub mod to_default;

pub use self::mode::ConformMode;
use self::{
    from_c_integer::from_c_integer, from_float::from_float, from_float_literal::from_float_literal,
    from_integer::from_integer, from_integer_literal::from_integer_literal,
    from_pointer::from_pointer,
};
use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    ast::ConformBehavior,
    resolved::{Type, TypeKind, TypedExpr},
    source_files::Source,
};

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
        TypeKind::CInteger(from_size, from_sign) => {
            from_c_integer(expr, mode, *from_size, *from_sign, to_type, conform_source)
        }
        _ => None,
    }
}

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
