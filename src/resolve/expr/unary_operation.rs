use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, Source},
    resolve::{
        conform_integer_to_default_or_error,
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    resolved::{self, TypedExpr},
};

pub fn resolve_unary_operation_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    unary_operation: &ast::UnaryOperation,
    preferred_type: Option<PreferredType>,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let resolved_expr = resolve_expr(
        ctx,
        &unary_operation.inner,
        preferred_type,
        Initialized::Require,
    )?;

    let resolved_expr = match &resolved_expr.resolved_type.kind {
        resolved::TypeKind::Boolean => resolved_expr,
        resolved::TypeKind::Integer { .. } => resolved_expr,
        resolved::TypeKind::IntegerLiteral(value) => {
            conform_integer_to_default_or_error(&value, resolved_expr.expr.source)?
        }
        _ => {
            return Err(ResolveErrorKind::CannotPerformUnaryOperationForType {
                operator: unary_operation.operator.to_string(),
                bad_type: resolved_expr.resolved_type.to_string(),
            }
            .at(source));
        }
    };

    let result_type = match unary_operation.operator {
        resolved::UnaryOperator::Not => resolved::TypeKind::Boolean.at(source),
        resolved::UnaryOperator::BitComplement => resolved_expr.resolved_type.clone(),
        resolved::UnaryOperator::Negate => resolved_expr.resolved_type.clone(),
    };

    let expr = resolved::Expr::new(
        resolved::ExprKind::UnaryOperation(Box::new(resolved::UnaryOperation {
            operator: unary_operation.operator.clone(),
            inner: resolved_expr,
        })),
        source,
    );

    Ok(TypedExpr::new(result_type, expr))
}
