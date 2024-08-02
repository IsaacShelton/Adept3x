use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, ConformBehavior},
    resolve::{
        conform_expr,
        error::{ResolveError, ResolveErrorKind},
        ConformMode, Initialized,
    },
    resolved::{self, Drops, TypedExpr},
    source_files::Source,
};

pub fn resolve_short_circuiting_binary_operation_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    binary_operation: &ast::ShortCircuitingBinaryOperation,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let local_bool_type = resolved::TypeKind::Boolean.at(source);
    let preferred_type = Some(PreferredType::of(&local_bool_type));

    let left = resolve_expr(
        ctx,
        &binary_operation.left,
        preferred_type,
        Initialized::Require,
    )?;

    let left = conform_expr(
        &left,
        &local_bool_type,
        ConformMode::Normal,
        ConformBehavior::Adept,
        source,
    )
    .ok_or_else(|| {
        ResolveErrorKind::ExpectedTypeForSide {
            side: "left-hand side".to_string(),
            operator: binary_operation.operator.to_string(),
            expected: resolved::TypeKind::Boolean.to_string(),
            got: left.resolved_type.to_string(),
        }
        .at(source)
    })?;

    ctx.variable_search_ctx.begin_scope();

    let right = resolve_expr(
        ctx,
        &binary_operation.right,
        preferred_type,
        Initialized::Require,
    )?;

    let right = conform_expr(
        &right,
        &local_bool_type,
        ConformMode::Normal,
        ConformBehavior::Adept,
        source,
    )
    .ok_or_else(|| {
        ResolveErrorKind::ExpectedTypeForSide {
            side: "right-hand side".to_string(),
            operator: binary_operation.operator.to_string(),
            expected: resolved::TypeKind::Boolean.to_string(),
            got: right.resolved_type.to_string(),
        }
        .at(source)
    })?;

    ctx.variable_search_ctx.end_scope();

    Ok(TypedExpr::new(
        local_bool_type,
        resolved::Expr::new(
            resolved::ExprKind::ShortCircuitingBinaryOperation(Box::new(
                resolved::ShortCircuitingBinaryOperation {
                    operator: binary_operation.operator,
                    left,
                    right,
                    drops: Drops::default(),
                },
            )),
            source,
        ),
    ))
}
