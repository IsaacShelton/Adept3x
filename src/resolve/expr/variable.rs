use super::{PreferredType, ResolveExprCtx};
use crate::{
    name::{Name, ResolvedName},
    resolve::{
        error::{ResolveError, ResolveErrorKind},
        expr::resolve_expr,
        Initialized,
    },
    resolved::{self, TypedExpr},
    source_files::Source,
};

pub fn resolve_variable_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    name: &str,
    preferred_type: Option<PreferredType>,
    initialized: Initialized,
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
                resolved::ExprKind::Variable(Box::new(resolved::Variable {
                    key: variable.key,
                    resolved_type: variable.resolved_type.clone(),
                })),
                source,
            ),
            is_initialized,
        ))
    } else if let Some((resolved_type, reference)) = ctx.global_search_ctx.find_global(name) {
        Ok(TypedExpr::new(
            resolved_type.clone(),
            resolved::Expr::new(
                resolved::ExprKind::GlobalVariable(Box::new(resolved::GlobalVariable {
                    reference: *reference,
                    resolved_type: resolved_type.clone(),
                })),
                source,
            ),
        ))
    } else if let Some(define) = ctx
        .helper_exprs
        // TODO: CLEANUP: PERFORMANCE: Once we have proper support for
        // namespaced helper expressions, this should be cleaned up
        .get(&ResolvedName::new(&Name::new(None::<&str>, name)))
    {
        let TypedExpr {
            resolved_type,
            expr,
            is_initialized,
        } = resolve_expr(ctx, &define.value, preferred_type, initialized)?;

        Ok(TypedExpr::new_maybe_initialized(
            resolved_type,
            resolved::Expr::new(
                resolved::ExprKind::ResolvedNamedExpression(name.to_string(), Box::new(expr)),
                source,
            ),
            is_initialized,
        ))
    } else {
        Err(ResolveErrorKind::UndeclaredVariable {
            name: name.to_string(),
        }
        .at(source))
    }
}
