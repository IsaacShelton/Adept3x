use super::{call::call_callee, resolve_expr, ResolveExprCtx, ResolveExprMode};
use crate::{
    asg::{self, TypeKind, TypedExpr},
    ast::{self, StaticMember, StaticMemberActionKind},
    resolve::{
        error::{ResolveError, ResolveErrorKind},
        func_haystack::{FindFunctionError, FuncHaystack},
        initialized::Initialized,
        PolyCatalog,
    },
    source_files::Source,
};
use itertools::Itertools;

pub fn resolve_static_member(
    ctx: &mut ResolveExprCtx,
    static_access: &StaticMember,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let subject = &static_access.subject;
    match &static_access.action.kind {
        StaticMemberActionKind::Value(member) => {
            resolve_static_member_value(ctx, subject, member, source)
        }
        StaticMemberActionKind::Call(call) => {
            resolve_static_member_call(ctx, subject, call, source)
        }
    }
}

pub fn resolve_static_member_value(
    ctx: &mut ResolveExprCtx,
    subject: &ast::Type,
    member: &str,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let ty = ctx.type_ctx().resolve(&subject)?;

    let TypeKind::Enum(human_name, enum_ref) = &ty.kind else {
        return Err(ResolveErrorKind::StaticMemberOfTypeDoesNotExist {
            ty: subject.to_string(),
            member: member.to_string(),
        }
        .at(source));
    };

    Ok(TypedExpr::new(
        ty.clone(),
        asg::Expr::new(
            asg::ExprKind::EnumMemberLiteral(Box::new(asg::EnumMemberLiteral {
                human_name: human_name.clone(),
                enum_ref: *enum_ref,
                variant_name: member.to_string(),
                source,
            })),
            source,
        ),
    ))
}

pub fn resolve_static_member_call(
    ctx: &mut ResolveExprCtx,
    subject: &ast::Type,
    call: &ast::Call,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let ast::TypeKind::Named(impl_name, impl_args) = &subject.kind else {
        return Err(ResolveError::other("Invalid implementation name", source));
    };

    let Some(impl_name) = impl_name.as_plain_str() else {
        return Err(ResolveError::other("Invalid implementation name", source));
    };

    let impl_ref = ctx
        .impls_in_modules
        .get(&ctx.module_fs_node_id)
        .and_then(|impls| impls.get(impl_name));

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

    let Some(imp) = impl_ref.and_then(|found| ctx.asg.impls.get(*found)) else {
        return Err(ResolveError::other(
            "Undefined trait implementation",
            source,
        ));
    };

    if imp.name_params.len() != impl_args.len() {
        return Err(ResolveError::other(
            "Wrong number of arguments for implementation",
            source,
        ));
    }

    let target = &imp.target;

    let Some(callee_name) = call.name.as_plain_str() else {
        return Err(ResolveError::other(
            "Implementation does not have namespaced functions",
            source,
        ));
    };

    let mut catalog = PolyCatalog::default();

    for (name, arg) in imp.name_params.keys().zip(impl_args) {
        match arg {
            ast::TypeArg::Type(ty) => {
                catalog
                    .put_type(name, &ctx.type_ctx().resolve(ty)?)
                    .expect("unique impl parameter names");
            }
            ast::TypeArg::Expr(expr) => {
                return Err(ResolveError::other(
                    "Cannot use expressions as implementation parameters yet",
                    expr.source,
                ));
            }
        }
    }

    let mut matches = imp
        .body
        .get(callee_name)
        .into_iter()
        .flatten()
        .flat_map(|func_ref| {
            FuncHaystack::fits(ctx, *func_ref, &args, Some(catalog.clone()), source)
        });

    let callee = matches
        .next()
        .map(|found| {
            if matches.next().is_some() {
                Err(FindFunctionError::Ambiguous)
            } else {
                Ok(found)
            }
        })
        .unwrap_or_else(|| Err(FindFunctionError::NotDefined))
        .map_err(|reason| {
            ResolveErrorKind::FailedToFindFunction {
                signature: format!(
                    "{}::{}({})",
                    impl_name,
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
