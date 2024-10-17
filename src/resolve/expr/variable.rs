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
    resolved::{self, GlobalVarDecl, Type, TypedExpr},
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

    if let Some(basename) = name.as_plain_str() {
        if let Some(found) = ctx
            .globals_in_modules
            .get(&ctx.module_fs_node_id)
            .and_then(|globals| globals.get(basename))
        {
            return Ok(resolve_global_variable(ctx, found, source));
        }
    }

    if !name.namespace.is_empty() {
        let Name {
            namespace,
            basename,
            ..
        } = name;

        let mut matches = ctx
            .settings
            .namespace_to_dependency
            .get(namespace.as_ref())
            .into_iter()
            .flatten()
            .flat_map(|dep| ctx.settings.dependency_to_module.get(dep))
            .flat_map(|fs_node_id| {
                ctx.globals_in_modules
                    .get(fs_node_id)
                    .and_then(|globals| globals.get(basename.as_ref()))
            })
            .filter(|decl| decl.privacy.is_public());

        if let Some(found) = matches.next() {
            if matches.next().is_some() {
                return Err(ResolveErrorKind::AmbiguousGlobal {
                    name: name.to_string(),
                }
                .at(source));
            }

            return Ok(resolve_global_variable(ctx, found, source));
        }
    }

    if let Some(helper_expr) = ctx.helper_exprs.get(&resolved_name) {
        return resolve_helper_expr(ctx, helper_expr, preferred_type, initialized, source);
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
    ctx: &ResolveExprCtx,
    decl: &GlobalVarDecl,
    source: Source,
) -> TypedExpr {
    let global = ctx
        .resolved_ast
        .globals
        .get(decl.global_ref)
        .expect("valid global");

    TypedExpr::new(
        global.resolved_type.clone(),
        resolved::Expr::new(
            resolved::ExprKind::GlobalVariable(Box::new(resolved::GlobalVariable {
                reference: decl.global_ref,
                resolved_type: global.resolved_type.clone(),
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
