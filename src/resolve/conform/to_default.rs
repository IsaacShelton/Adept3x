use super::from_integer_literal::from_integer_literal;
use crate::{
    ast::{CIntegerAssumptions, FloatSize},
    resolve::error::{ResolveError, ResolveErrorKind},
    resolved::{Expr, ExprKind, TypeKind, TypedExpr},
    source_files::Source,
};
use num::BigInt;

pub fn conform_expr_to_default(
    expr: TypedExpr,
    c_integer_assumptions: CIntegerAssumptions,
) -> Result<TypedExpr, ResolveError> {
    match &expr.resolved_type.kind {
        TypeKind::IntegerLiteral(value) => conform_integer_literal_to_default_or_error(
            value,
            c_integer_assumptions,
            expr.expr.source,
        ),
        TypeKind::FloatLiteral(value) => {
            Ok(conform_float_literal_to_default(*value, expr.expr.source))
        }
        _ => Ok(expr),
    }
}

pub fn conform_float_literal_to_default(value: f64, source: Source) -> TypedExpr {
    TypedExpr::new(
        TypeKind::f64().at(source),
        Expr::new(ExprKind::FloatingLiteral(FloatSize::Bits64, value), source),
    )
}

pub fn conform_integer_literal_to_default_or_error(
    value: &BigInt,
    c_integer_assumptions: CIntegerAssumptions,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    conform_integer_literal_to_default(value, c_integer_assumptions, source).ok_or_else(|| {
        ResolveErrorKind::UnrepresentableInteger {
            value: value.to_string(),
        }
        .at(source)
    })
}

pub fn conform_integer_literal_to_default(
    value: &BigInt,
    c_integer_assumptions: CIntegerAssumptions,
    source: Source,
) -> Option<TypedExpr> {
    for possible_type in [
        TypeKind::i32().at(source),
        TypeKind::u32().at(source),
        TypeKind::i64().at(source),
        TypeKind::u64().at(source),
    ] {
        if let Some(fit) =
            from_integer_literal(value, c_integer_assumptions, source, &possible_type)
        {
            return Some(fit);
        }
    }

    None
}
