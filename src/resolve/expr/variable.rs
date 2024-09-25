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
    name: &Name,
    preferred_type: Option<PreferredType>,
    initialized: Initialized,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    if let Some(variable) = name
        .as_plain_str()
        .and_then(|name| ctx.variable_search_ctx.find_variable(name))
    {
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

        return Ok(TypedExpr::new_maybe_initialized(
            variable.resolved_type.clone(),
            resolved::Expr::new(
                resolved::ExprKind::Variable(Box::new(resolved::Variable {
                    key: variable.key,
                    resolved_type: variable.resolved_type.clone(),
                })),
                source,
            ),
            is_initialized,
        ));
    }

    let resolved_name = ResolvedName::new(name);

    if let Some((resolved_type, reference)) = ctx.global_search_ctx.find_global(&resolved_name) {
        return Ok(TypedExpr::new(
            resolved_type.clone(),
            resolved::Expr::new(
                resolved::ExprKind::GlobalVariable(Box::new(resolved::GlobalVariable {
                    reference: *reference,
                    resolved_type: resolved_type.clone(),
                })),
                source,
            ),
        ));
    }

    if let Some(define) = ctx.helper_exprs.get(&resolved_name) {
        let TypedExpr {
            resolved_type,
            expr,
            is_initialized,
        } = resolve_expr(ctx, &define.value, preferred_type, initialized)?;

        return Ok(TypedExpr::new_maybe_initialized(
            resolved_type,
            resolved::Expr::new(
                resolved::ExprKind::ResolvedNamedExpression(name.to_string(), Box::new(expr)),
                source,
            ),
            is_initialized,
        ));
    }

    // TODO: Check if any global variables from imported namespaces match
    // TODO: Check if any helper exprs from imported namespaces match
    // TODO: They should probably be checked at the same time

    Err(ResolveErrorKind::UndeclaredVariable {
        name: name.to_string(),
    }
    .at(source))
}
