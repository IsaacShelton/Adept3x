mod cast;
mod infer_impl;
mod specified_impl;

use super::{PreferredType, ResolveExprCtx, ResolveExprMode, resolve_expr};
use crate::{
    conform::{ConformMode, Perform, conform_expr, to_default::conform_expr_to_default},
    error::{ResolveError, ResolveErrorKind},
    initialized::Initialized,
    resolve_type_args_to_poly_args,
    type_ctx::ResolveTypeOptions,
};
use asg::{Callee, IntoPolyRecipeResolver, PolyCatalog, TypedExpr};
use cast::find_builtin_cast_func;
use infer_impl::infer_callee_missing_impl_args;
use itertools::Itertools;
use source_files::Source;
use specified_impl::resolve_impl_arg;
use std::collections::HashSet;

pub fn resolve_call_expr(
    ctx: &mut ResolveExprCtx,
    call: &ast::Call,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let mut args = Vec::with_capacity(call.args.len());
    for arg in call.args.iter() {
        args.push(resolve_expr(
            ctx,
            arg,
            None,
            Initialized::Require,
            ResolveExprMode::RequireValue,
        )?);
    }

    let generics = resolve_type_args_to_poly_args(ctx, &call.generics)?;

    let args = match find_builtin_cast_func(ctx, call, args, source)? {
        Ok(cast) => return Ok(cast),
        Err(args) => args,
    };

    let callee = ctx
        .func_haystack
        .find(ctx, &call.name, &generics, &args[..], source)
        .map_err(|reason| {
            ResolveErrorKind::FailedToFindFunction {
                signature: format!(
                    "{}({})",
                    call.name,
                    args.iter().map(|arg| arg.ty.to_string()).join(", ")
                ),
                reason,
                almost_matches: ctx.func_haystack.find_near_matches(ctx, &call.name),
            }
            .at(source)
        })?;

    call_callee(ctx, call, callee, args, source)
}

pub fn call_callee(
    ctx: &mut ResolveExprCtx,
    call: &ast::Call,
    mut callee: Callee,
    mut args: Vec<TypedExpr>,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let mut used_names = HashSet::new();
    let mut catalog = PolyCatalog {
        polymorphs: callee.recipe.polymorphs.clone(),
    };

    for using in call.using.iter() {
        resolve_impl_arg(ctx, &mut callee, using, &mut used_names, &mut catalog)?;
    }

    let function = &ctx.asg.funcs[callee.func_ref];
    let num_required = function.params.required.len();

    infer_callee_missing_impl_args(ctx, function, &mut used_names, &mut catalog, source)?;

    // We shouldn't use used_names after this, since we know all names were satisfied
    drop(used_names);

    callee.recipe = catalog.bake();

    for (i, arg) in args.iter_mut().enumerate() {
        let function = &ctx.asg.funcs[callee.func_ref];

        let preferred_type =
            (i < num_required).then_some(PreferredType::of_parameter(callee.func_ref, i));

        if preferred_type.map_or(false, |ty| ty.view(&ctx.asg).kind.contains_polymorph()) {
            *arg = conform_expr_to_default::<Perform>(&*arg, ctx.c_integer_assumptions())
                .map_err(|_| ResolveErrorKind::FailedToConformArgumentToDefaultValue.at(source))?;
            continue;
        }

        *arg = preferred_type
            .map(|preferred_type| {
                let preferred_type = preferred_type.view(ctx.asg);
                conform_expr::<Perform>(
                    ctx,
                    &arg,
                    preferred_type,
                    ConformMode::ParameterPassing,
                    ctx.adept_conform_behavior(),
                    source,
                )
                .map_err(|_| {
                    ResolveErrorKind::BadTypeForArgumentToFunction {
                        expected: preferred_type.to_string(),
                        got: arg.ty.to_string(),
                        name: function.name.display(&ctx.asg.workspace.fs).to_string(),
                        i,
                    }
                    .at(source)
                })
            })
            .unwrap_or_else(|| {
                conform_expr_to_default::<Perform>(&*arg, ctx.c_integer_assumptions())
                    .map_err(|_| ResolveErrorKind::FailedToConformArgumentToDefaultValue.at(source))
            })?;
    }

    let return_type = callee
        .recipe
        .resolver()
        .resolve_type(&function.return_type)
        .map_err(ResolveError::from)?;

    if let Some(required_ty) = &call.expected_to_return {
        let resolved_required_ty = ctx
            .type_ctx()
            .resolve(required_ty, ResolveTypeOptions::Unalias)?;

        if resolved_required_ty != return_type {
            return Err(ResolveErrorKind::FunctionMustReturnType {
                of: required_ty.to_string(),
                func_name: function.name.display(&ctx.asg.workspace.fs).to_string(),
            }
            .at(function.return_type.source));
        }
    }

    Ok(TypedExpr::new(
        return_type,
        asg::Expr::new(
            asg::ExprKind::Call(Box::new(asg::Call { callee, args })),
            source,
        ),
    ))
}
