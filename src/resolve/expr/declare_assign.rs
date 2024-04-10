use super::{resolve_expr, ResolveExprCtx};
use crate::{
    ast::{self, Source},
    resolve::{conform_expr_to_default, error::ResolveError, Initialized},
    resolved::{self, TypedExpr},
};

pub fn resolve_declare_assign_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    declare_assign: &ast::DeclareAssign,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let value = resolve_expr(ctx, &declare_assign.value, Initialized::Require)?;

    let value = conform_expr_to_default(value, ctx.resolved_ast.source_file_cache)?;

    let function = ctx
        .resolved_ast
        .functions
        .get_mut(ctx.resolved_function_ref)
        .unwrap();

    let key = function
        .variables
        .add_variable(value.resolved_type.clone(), true);

    ctx.variable_search_ctx
        .put(&declare_assign.name, value.resolved_type.clone(), key);

    Ok(TypedExpr::new(
        value.resolved_type.clone(),
        resolved::Expr::new(
            resolved::ExprKind::DeclareAssign(resolved::DeclareAssign {
                key,
                value: Box::new(value.expr),
                resolved_type: value.resolved_type,
            }),
            source,
        ),
    ))
}
