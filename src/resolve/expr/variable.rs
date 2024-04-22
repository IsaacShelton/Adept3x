use super::ResolveExprCtx;
use crate::{
    ast::Source,
    resolve::error::ResolveError,
    resolved::{self, TypedExpr},
};

pub fn resolve_variable_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    name: &str,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    if let Some(variable) = ctx.variable_search_ctx.find_variable(name) {
        let function = ctx
            .resolved_ast
            .functions
            .get_mut(ctx.resolved_function_ref)
            .unwrap();

        let is_initialized = function
            .variables
            .get(variable.key)
            .expect("found variable to exist")
            .is_initialized();

        Ok(TypedExpr::new_maybe_initialized(
            variable.resolved_type.clone(),
            resolved::Expr::new(
                resolved::ExprKind::Variable(resolved::Variable {
                    key: variable.key,
                    resolved_type: variable.resolved_type.clone(),
                }),
                source,
            ),
            is_initialized,
        ))
    } else {
        let (resolved_type, reference) =
            ctx.global_search_ctx.find_global_or_error(name, source)?;

        Ok(TypedExpr::new(
            resolved_type.clone(),
            resolved::Expr::new(
                resolved::ExprKind::GlobalVariable(resolved::GlobalVariable {
                    reference: *reference,
                    resolved_type: resolved_type.clone(),
                }),
                source,
            ),
        ))
    }
}
