use super::{PreferredType, ResolveExprCtx};
use crate::{
    ast::HelperExpr,
    ir::GlobalVarRef,
    name::Name,
    resolve::{
        error::{ResolveError, ResolveErrorKind},
        expr::resolve_expr,
        Initialized,
    },
    asg::{self, GlobalVarDecl, Type, TypedExpr},
    source_files::Source,
};

pub fn resolve_variable_expr(
    ctx: &mut ResolveExprCtx,
    name: &Name,
    _preferred_type: Option<PreferredType>,
    _initialized: Initialized,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    if let Some(variable) = name
        .as_plain_str()
        .and_then(|name| ctx.variable_haystack.find(name))
    {
        if let Some(function) = ctx.resolved_function_ref.map(|function_ref| {
            ctx.asg
                .functions
                .get_mut(function_ref)
                .expect("valid function ref")
        }) {
            let is_initialized = function
                .variables
                .get(variable.key)
                .expect("found variable to exist")
                .is_initialized();

            return Ok(TypedExpr::new_maybe_initialized(
                variable.resolved_type.clone(),
                asg::Expr::new(
                    asg::ExprKind::Variable(Box::new(asg::Variable {
                        key: variable.key,
                        resolved_type: variable.resolved_type.clone(),
                    })),
                    source,
                ),
                is_initialized,
            ));
        }
    }

    if let Some(basename) = name.as_plain_str() {
        let maybe_global = ctx
            .globals_in_modules
            .get(&ctx.module_fs_node_id)
            .and_then(|globals| globals.get(basename));

        let maybe_helper_expr = ctx
            .helper_exprs_in_modules
            .get(&ctx.module_fs_node_id)
            .and_then(|helper_expr| helper_expr.get(basename));

        if maybe_global.is_some() && maybe_helper_expr.is_some() {
            return Err(ResolveErrorKind::AmbiguousSymbol {
                name: basename.into(),
            }
            .at(source));
        }

        if let Some(found) = maybe_global {
            return Ok(resolve_global_variable(ctx, found, source));
        }

        if let Some(found) = maybe_helper_expr {
            return Ok(found.value.clone());
        }
    }

    if !name.namespace.is_empty() {
        let Name {
            namespace,
            basename,
            ..
        } = name;

        let modules = ctx
            .settings
            .namespace_to_dependency
            .get(namespace.as_ref())
            .into_iter()
            .flatten()
            .flat_map(|dep| ctx.settings.dependency_to_module.get(dep));

        let mut global_var_matches = modules
            .clone()
            .flat_map(|fs_node_id| {
                ctx.globals_in_modules
                    .get(fs_node_id)
                    .and_then(|globals| globals.get(basename.as_ref()))
            })
            .filter(|decl| decl.privacy.is_public());

        let mut helper_expr_matches = modules
            .flat_map(|fs_node_id| {
                ctx.helper_exprs_in_modules
                    .get(fs_node_id)
                    .and_then(|helper_expr| helper_expr.get(basename.as_ref()))
            })
            .filter(|decl| decl.privacy.is_public());

        let maybe_global = global_var_matches.next();
        let maybe_helper_expr = helper_expr_matches.next();

        if maybe_global.is_some() && maybe_helper_expr.is_some() {
            return Err(ResolveErrorKind::AmbiguousSymbol {
                name: basename.to_string(),
            }
            .at(source));
        }

        if let Some(found) = maybe_global {
            if global_var_matches.next().is_some() {
                return Err(ResolveErrorKind::AmbiguousGlobal {
                    name: name.to_string(),
                }
                .at(source));
            }

            return Ok(resolve_global_variable(ctx, found, source));
        }

        if let Some(found) = maybe_helper_expr {
            if helper_expr_matches.next().is_some() {
                return Err(ResolveErrorKind::AmbiguousHelperExpr {
                    name: name.to_string(),
                }
                .at(source));
            }

            return Ok(found.value.clone());
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
    ctx: &ResolveExprCtx,
    decl: &GlobalVarDecl,
    source: Source,
) -> TypedExpr {
    let global = ctx
        .asg
        .globals
        .get(decl.global_ref)
        .expect("valid global");

    TypedExpr::new(
        global.resolved_type.clone(),
        asg::Expr::new(
            asg::ExprKind::GlobalVariable(Box::new(asg::GlobalVariable {
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
        asg::Expr::new(
            asg::ExprKind::ResolvedNamedExpression(Box::new(expr)),
            source,
        ),
        is_initialized,
    ));
}
