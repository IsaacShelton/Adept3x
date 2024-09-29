use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, UnaryMathOperator},
    resolve::{
        conform::to_default::conform_expr_to_default_or_error,
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    resolved::{Expr, ExprKind, TypeKind, TypedExpr, UnaryMathOperation},
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

    if resolved_expr.resolved_type.is_ambiguous() {
        return Err(ResolveErrorKind::CannotPerformUnaryOperationForType {
            operator: operator.to_string(),
            bad_type: resolved_expr.resolved_type.to_string(),
        }
        .at(source));
    }

    let result_type = match operator {
        UnaryMathOperator::Not | UnaryMathOperator::IsNonZero => TypeKind::Boolean.at(source),
        UnaryMathOperator::BitComplement | UnaryMathOperator::Negate => {
            resolved_expr.resolved_type.clone()
        }
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
