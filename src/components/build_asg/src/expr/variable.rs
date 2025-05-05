use super::{PreferredType, ResolveExprCtx, ResolveExprMode, resolve_expr};
use crate::{
    error::{ResolveError, ResolveErrorKind},
    initialized::Initialized,
};
use asg::{GlobalDecl, TypedExpr};
use ast::Name;
use source_files::Source;

pub fn resolve_variable_expr(
    ctx: &mut ResolveExprCtx,
    name: &Name,
    preferred_type: Option<PreferredType>,
    initialized: Initialized,
    mode: ResolveExprMode,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    if let Some(variable) = name
        .as_plain_str()
        .and_then(|name| ctx.variable_haystack.find(name))
    {
        if ctx.func_ref.is_some() {
            return Ok(TypedExpr::new(
                variable.ty.clone(),
                asg::Expr::new(
                    asg::ExprKind::Variable(Box::new(asg::Variable {
                        key: variable.key,
                        ty: variable.ty.clone(),
                    })),
                    source,
                ),
            ));
        }
    }

    if let Some(basename) = name.as_plain_str() {
        let maybe_global = ctx
            .globals_in_modules
            .get(&ctx.physical_fs_node_id)
            .or_else(|| ctx.globals_in_modules.get(&ctx.module_fs_node_id))
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
            return resolve_expr(ctx, &found.value, preferred_type, initialized, mode);
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

            return resolve_expr(ctx, &found.value, preferred_type, initialized, mode);
        }
    }

    Err(ResolveErrorKind::UndeclaredVariable {
        name: name.to_string(),
    }
    .at(source))
}

fn resolve_global_variable(ctx: &ResolveExprCtx, decl: &GlobalDecl, source: Source) -> TypedExpr {
    let global = &ctx.asg.globals[decl.global_ref];

    TypedExpr::new(
        global.ty.clone(),
        asg::Expr::new(
            asg::ExprKind::GlobalVariable(Box::new(asg::GlobalVariable {
                reference: decl.global_ref,
                ty: global.ty.clone(),
            })),
            source,
        ),
    )
}
