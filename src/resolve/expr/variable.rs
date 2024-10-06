use super::{PreferredType, ResolveExprCtx};
use crate::{
    ast::HelperExpr,
    ir::GlobalVarRef,
    name::{Name, ResolvedName},
    resolve::{
        error::{ResolveError, ResolveErrorKind},
        expr::resolve_expr,
        Initialized,
    },
    resolved::{self, Type, TypedExpr},
    source_files::Source,
};

pub fn resolve_variable_expr(
    ctx: &mut ResolveExprCtx,
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

    let resolved_name = ResolvedName::new(ctx.module_fs_node_id, name);

    if let Some((resolved_type, reference)) = ctx.global_search_ctx.find_global(&resolved_name) {
        return Ok(resolve_global_variable(resolved_type, *reference, source));
    }

    if let Some(helper_expr) = ctx.helper_exprs.get(&resolved_name) {
        return resolve_helper_expr(ctx, helper_expr, preferred_type, initialized, source);
    }

    if name.namespace.is_empty() {
        let mut matches = ctx
            .settings
            .imported_namespaces
            .iter()
            .flat_map(|namespace| {
                let resolved_name = ResolvedName::new(
                    ctx.module_fs_node_id,
                    &Name::new(Some(namespace.to_string()), name.basename.to_string()),
                );

                let global = ctx
                    .global_search_ctx
                    .find_global(&resolved_name)
                    .map(GlobalOrHelper::Global);

                let helper_expr = ctx
                    .helper_exprs
                    .get(&resolved_name)
                    .copied()
                    .map(GlobalOrHelper::HelperExpr);

                [global, helper_expr]
            })
            .flatten();

        if let Some(found) = matches.next() {
            if matches.next().is_some() {
                return Err(ResolveErrorKind::AmbiguousSymbol {
                    name: name.to_string(),
                }
                .at(source));
            }

            return match found {
                GlobalOrHelper::Global((ty, reference)) => {
                    Ok(resolve_global_variable(ty, *reference, source))
                }
                GlobalOrHelper::HelperExpr(helper_expr) => {
                    resolve_helper_expr(ctx, helper_expr, preferred_type, initialized, source)
                }
            };
        }
    }

    Err(ResolveErrorKind::UndeclaredVariable {
        name: name.to_string(),
    }
    .at(source))
}

enum GlobalOrHelper<'a> {
    Global((&'a Type, &'a GlobalVarRef)),
    HelperExpr(&'a HelperExpr),
}

fn resolve_global_variable(
    resolved_type: &Type,
    reference: GlobalVarRef,
    source: Source,
) -> TypedExpr {
    TypedExpr::new(
        resolved_type.clone(),
        resolved::Expr::new(
            resolved::ExprKind::GlobalVariable(Box::new(resolved::GlobalVariable {
                reference,
                resolved_type: resolved_type.clone(),
            })),
            source,
        ),
    )
}

fn resolve_helper_expr(
    ctx: &mut ResolveExprCtx,
    helper_expr: &HelperExpr,
    preferred_type: Option<PreferredType>,
    initialized: Initialized,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let TypedExpr {
        resolved_type,
        expr,
        is_initialized,
    } = resolve_expr(ctx, &helper_expr.value, preferred_type, initialized)?;

    return Ok(TypedExpr::new_maybe_initialized(
        resolved_type,
        resolved::Expr::new(
            resolved::ExprKind::ResolvedNamedExpression(Box::new(expr)),
            source,
        ),
        is_initialized,
    ));
}
