use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, UnaryMathOperator},
    resolve::{
        conform::to_default::conform_expr_to_default_or_error,
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    asg::{Expr, ExprKind, TypeKind, TypedExpr, UnaryMathOperation},
    source_files::Source,
};

pub fn resolve_unary_math_operation_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    operator: &UnaryMathOperator,
    inner: &ast::Expr,
    preferred_type: Option<PreferredType>,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let resolved_expr = resolve_expr(ctx, inner, preferred_type, Initialized::Require)
        .and_then(|expr| conform_expr_to_default_or_error(expr, ctx.c_integer_assumptions()))?;

    let from_type = &resolved_expr.ty;

    if from_type.is_ambiguous() {
        return Err(ResolveErrorKind::CannotPerformUnaryOperationForType {
            operator: operator.to_string(),
            bad_type: from_type.to_string(),
        }
        .at(source));
    }

    let result_type = match operator {
        UnaryMathOperator::Not => (from_type.kind.is_boolean() || from_type.kind.is_integer_like())
            .then(|| TypeKind::Boolean.at(source)),
        UnaryMathOperator::IsNonZero => (from_type.kind.is_boolean()
            || from_type.kind.is_integer_like()
            || from_type.kind.is_float_like())
        .then(|| TypeKind::Boolean.at(source)),
        UnaryMathOperator::Negate => (from_type.kind.is_integer_like()
            || from_type.kind.is_float_like())
        .then(|| from_type.clone()),
        UnaryMathOperator::BitComplement => {
            (from_type.kind.is_integer_like()).then(|| from_type.clone())
        }
    };

    let Some(result_type) = result_type else {
        return Err(ResolveErrorKind::CannotPerformUnaryOperationForType {
            operator: operator.to_string(),
            bad_type: from_type.to_string(),
        }
        .at(source));
    };

    Ok(TypedExpr::new(
        result_type,
        Expr::new(
            ExprKind::UnaryMathOperation(Box::new(UnaryMathOperation {
                operator: operator.clone(),
                inner: resolved_expr,
            })),
            source,
        ),
    ))
}
