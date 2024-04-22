use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, Source},
    resolve::{
        conform_expr, error::{ResolveError, ResolveErrorKind}, ConformMode, Initialized
    },
    resolved::{self, TypedExpr},
};

pub fn resolve_short_circuiting_binary_operation_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    binary_operation: &ast::ShortCircuitingBinaryOperation,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let preferred_type = Some(PreferredType::of(&resolved::Type::Boolean));

    let left = resolve_expr(
        ctx,
        &binary_operation.left,
        preferred_type,
        Initialized::Require,
    )?;

    let right = resolve_expr(
        ctx,
        &binary_operation.right,
        preferred_type,
        Initialized::Require,
    )?;

    let left = conform_expr(&left, &resolved::Type::Boolean, ConformMode::Normal).ok_or_else(|| {
        ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::ExpectedTypeForSide {
                side: "left-hand side".to_string(),
                operator: binary_operation.operator.to_string(),
                expected: resolved::Type::Boolean.to_string(),
                got: left.resolved_type.to_string(),
            },
        )
    })?;

    let right = conform_expr(&right, &resolved::Type::Boolean, ConformMode::Normal).ok_or_else(|| {
        ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::ExpectedTypeForSide {
                side: "right-hand side".to_string(),
                operator: binary_operation.operator.to_string(),
                expected: resolved::Type::Boolean.to_string(),
                got: right.resolved_type.to_string(),
            },
        )
    })?;

    Ok(TypedExpr::new(
        resolved::Type::Boolean,
        resolved::Expr::new(
            resolved::ExprKind::ShortCircuitingBinaryOperation(Box::new(
                resolved::ShortCircuitingBinaryOperation {
                    operator: binary_operation.operator,
                    left,
                    right,
                },
            )),
            source,
        ),
    ))
}
