use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, UnaryOperator},
    resolve::{
        conform::to_default::conform_integer_literal_to_default_or_error,
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    resolved::{Expr, ExprKind, TypeKind, TypedExpr, UnaryOperation},
    source_files::Source,
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
        TypeKind::Boolean => resolved_expr,
        TypeKind::Integer(..) => resolved_expr,
        TypeKind::IntegerLiteral(value) => {
            conform_integer_literal_to_default_or_error(value, resolved_expr.expr.source)?
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
        UnaryOperator::Not | UnaryOperator::IsNonZero => TypeKind::Boolean.at(source),
        UnaryOperator::BitComplement | UnaryOperator::Negate => resolved_expr.resolved_type.clone(),
    };

    let expr = Expr::new(
        ExprKind::UnaryOperation(Box::new(UnaryOperation {
            operator: unary_operation.operator.clone(),
            inner: resolved_expr,
        })),
        source,
    );

    Ok(TypedExpr::new(result_type, expr))
}
