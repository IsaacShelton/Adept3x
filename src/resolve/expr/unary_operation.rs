use crate::{ast::{self, Source}, resolve::{conform_integer_to_default_or_error, error::{ResolveError, ResolveErrorKind}, Initialized}, resolved::{self, TypedExpr}};
use super::{resolve_expr, PreferredType, ResolveExprCtx};

pub fn resolve_unary_operation_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    unary_operation: &ast::UnaryOperation,
    preferred_type: Option<PreferredType>,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let resolved_expr = resolve_expr(ctx, &unary_operation.inner, preferred_type, Initialized::Require)?;

    let resolved_expr = match resolved_expr.resolved_type {
        resolved::Type::Boolean => resolved_expr,
        resolved::Type::Integer { .. } => resolved_expr,
        resolved::Type::IntegerLiteral(value) => conform_integer_to_default_or_error(
            ctx.resolved_ast.source_file_cache,
            &value,
            resolved_expr.expr.source,
        )?,
        _ => {
            return Err(ResolveError::new(
                ctx.resolved_ast.source_file_cache,
                source,
                ResolveErrorKind::CannotPerformUnaryOperationForType {
                    operator: unary_operation.operator.to_string(),
                    bad_type: resolved_expr.resolved_type.to_string(),
                },
            ));
        }
    };

    let result_type = match unary_operation.operator {
        resolved::UnaryOperator::Not => resolved::Type::Boolean,
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
