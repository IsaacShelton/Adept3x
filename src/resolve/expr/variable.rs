use super::ResolveExpressionCtx;
use crate::{
    ast::Source,
    resolve::error::ResolveError,
    resolved::{self, TypedExpression},
};

pub fn resolve_variable_expression(
    ctx: &mut ResolveExpressionCtx<'_, '_>,
    name: &str,
    source: Source,
) -> Result<TypedExpression, ResolveError> {
    if let Some((resolved_type, key)) = ctx.variable_search_ctx.find_variable(name) {
        let function = ctx
            .resolved_ast
            .functions
            .get_mut(ctx.resolved_function_ref)
            .unwrap();

        let is_initialized = function
            .variables
            .get(*key)
            .expect("found variable to exist")
            .is_initialized();

        Ok(TypedExpression::new_maybe_initialized(
            resolved_type.clone(),
            resolved::Expression::new(
                resolved::ExpressionKind::Variable(resolved::Variable {
                    key: *key,
                    resolved_type: resolved_type.clone(),
                }),
                source,
            ),
            is_initialized,
        ))
    } else {
        let (resolved_type, reference) =
            ctx.global_search_ctx.find_global_or_error(name, source)?;

        Ok(TypedExpression::new(
            resolved_type.clone(),
            resolved::Expression::new(
                resolved::ExpressionKind::GlobalVariable(resolved::GlobalVariable {
                    reference: *reference,
                    resolved_type: resolved_type.clone(),
                }),
                source,
            ),
        ))
    }
}
