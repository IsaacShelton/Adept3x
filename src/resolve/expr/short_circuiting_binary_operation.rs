use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast,
    resolve::{
        conform::{conform_expr, ConformMode, Perform},
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    asg::{self, TypedExpr},
    source_files::Source,
};

pub fn resolve_short_circuiting_binary_operation_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    binary_operation: &ast::ShortCircuitingBinaryOperation,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let local_bool_type = asg::TypeKind::Boolean.at(source);
    let preferred_type = Some(PreferredType::of(&local_bool_type));

    let left = resolve_expr(
        ctx,
        &binary_operation.left,
        preferred_type,
        Initialized::Require,
    )?;

    let left = conform_expr::<Perform>(
        ctx,
        &left,
        &local_bool_type,
        ConformMode::Normal,
        ctx.adept_conform_behavior(),
        source,
    )
    .map_err(|_| {
        ResolveErrorKind::ExpectedTypeForSide {
            side: "left-hand side".to_string(),
            operator: binary_operation.operator.to_string(),
            expected: asg::TypeKind::Boolean.to_string(),
            got: left.ty.to_string(),
        }
        .at(source)
    })?;

    ctx.variable_haystack.begin_scope();

    let right = resolve_expr(
        ctx,
        &binary_operation.right,
        preferred_type,
        Initialized::Require,
    )?;

    let right = conform_expr::<Perform>(
        ctx,
        &right,
        &local_bool_type,
        ConformMode::Normal,
        ctx.adept_conform_behavior(),
        source,
    )
    .map_err(|_| {
        ResolveErrorKind::ExpectedTypeForSide {
            side: "right-hand side".to_string(),
            operator: binary_operation.operator.to_string(),
            expected: asg::TypeKind::Boolean.to_string(),
            got: right.ty.to_string(),
        }
        .at(source)
    })?;

    ctx.variable_haystack.end_scope();

    Ok(TypedExpr::new(
        local_bool_type,
        asg::Expr::new(
            asg::ExprKind::ShortCircuitingBinaryOperation(Box::new(
                asg::ShortCircuitingBinaryOperation {
                    operator: binary_operation.operator,
                    left,
                    right,
                },
            )),
            source,
        ),
    ))
}
