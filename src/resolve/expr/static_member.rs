use super::{call::call_callee, resolve_expr, ResolveExprCtx, ResolveExprMode};
use crate::{
    asg::{self, PolyCall, PolyCallee, TypeKind, TypedExpr},
    ast::{self, StaticMemberCall, StaticMemberValue, TypeArg},
    name::Name,
    resolve::{
        conform::{
            conform_expr, to_default::conform_expr_to_default, ConformMode, Perform, Validate,
        },
        error::{ResolveError, ResolveErrorKind},
        expr::PreferredType,
        func_haystack::{FindFunctionError, FuncHaystack},
        initialized::Initialized,
        PolyCatalog, PolyRecipe,
    },
};
use indexmap::IndexMap;
use itertools::Itertools;

pub fn resolve_static_member_value(
    ctx: &mut ResolveExprCtx,
    static_member_value: &StaticMemberValue,
) -> Result<TypedExpr, ResolveError> {
    let StaticMemberValue {
        subject,
        value,
        value_source,
        source,
    } = static_member_value;

    let ty = ctx.type_ctx().resolve(&subject)?;

    let TypeKind::Enum(human_name, enum_ref) = &ty.kind else {
        return Err(ResolveErrorKind::StaticMemberOfTypeDoesNotExist {
            ty: subject.to_string(),
            member: value.to_string(),
        }
        .at(*value_source));
    };

    Ok(TypedExpr::new(
        ty.clone(),
        asg::Expr::new(
            asg::ExprKind::EnumMemberLiteral(Box::new(asg::EnumMemberLiteral {
                human_name: human_name.clone(),
                enum_ref: *enum_ref,
                variant_name: value.to_string(),
                source: *value_source,
            })),
            *source,
        ),
    ))
}

pub fn resolve_static_member_call(
    ctx: &mut ResolveExprCtx,
    static_member_call: &StaticMemberCall,
) -> Result<TypedExpr, ResolveError> {
    match &static_member_call.subject.kind {
        ast::TypeKind::Named(impl_name, impl_args) => {
            resolve_static_member_call_named(ctx, static_member_call, impl_name, impl_args)
        }
        ast::TypeKind::Polymorph(polymorph, _constraints) => {
            resolve_static_member_call_polymorph(ctx, static_member_call, polymorph)
        }
        _ => Err(ResolveError::other(
            "Using callee supplied trait implementations is not supported yet",
            static_member_call.source,
        )),
    }
}

pub fn resolve_static_member_call_named(
    ctx: &mut ResolveExprCtx,
    static_member_call: &StaticMemberCall,
    impl_name: &Name,
    impl_args: &[TypeArg],
) -> Result<TypedExpr, ResolveError> {
    let StaticMemberCall {
        subject: _,
        call,
        call_source,
        source,
    } = &static_member_call;

    let Some(impl_name) = impl_name.as_plain_str() else {
        return Err(ResolveError::other("Invalid implementation name", *source));
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
            *source,
        ));
    };

    if imp.name_params.len() != impl_args.len() {
        return Err(ResolveError::other(
            "Wrong number of arguments for implementation",
            *source,
        ));
    }

    let target = &imp.target;

    let Some(callee_name) = call.name.as_plain_str() else {
        return Err(ResolveError::other(
            "Implementation does not have namespaced functions",
            *call_source,
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

    let mut only_match = imp.body.get(callee_name).into_iter().flat_map(|func_ref| {
        FuncHaystack::fits(ctx, *func_ref, &args, Some(catalog.clone()), *call_source)
    });

    let callee = only_match.next().ok_or_else(|| {
        ResolveErrorKind::FailedToFindFunction {
            signature: format!(
                "{}::{}({})",
                impl_name,
                call.name,
                args.iter().map(|arg| arg.ty.to_string()).join(", ")
            ),
            reason: FindFunctionError::NotDefined,
            almost_matches: vec![],
        }
        .at(*call_source)
    })?;

    call_callee(ctx, call, callee, args, *call_source)
}

pub fn resolve_static_member_call_polymorph(
    ctx: &mut ResolveExprCtx,
    static_member_call: &StaticMemberCall,
    polymorph: &str,
) -> Result<TypedExpr, ResolveError> {
    let StaticMemberCall {
        subject: _,
        call,
        call_source,
        source,
    } = static_member_call;

    let Some(func_ref) = ctx.func_ref else {
        return Err(ResolveError::other(
            "Cannot use implementation polymorph outside of function",
            *source,
        ));
    };

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

    let func = ctx
        .asg
        .funcs
        .get(func_ref)
        .expect("referenced function to exist");

    let Some(generic_trait_ref) = func.impl_params.params.get(polymorph) else {
        return Err(ResolveError::other(
            format!("Undeclared implementation '${}'", polymorph),
            *source,
        ));
    };

    let trait_decl = ctx
        .asg
        .traits
        .get(generic_trait_ref.trait_ref)
        .expect("referenced trait to exist");

    let member = call
        .name
        .as_plain_str()
        .ok_or_else(|| {
            ResolveError::other(
                "Namespaced functions do not exist on trait implementations",
                *call_source,
            )
        })?
        .to_string();

    let Some(trait_func) = trait_decl.funcs.get(&member) else {
        return Err(ResolveError::other(
            format!(
                "Function '{}' does not exist on trait '{}'",
                &member, &trait_decl.human_name.0
            ),
            *call_source,
        ));
    };

    if trait_decl.params.len() != generic_trait_ref.args.len() {
        return Err(ResolveError::other(
            format!(
                "Incorrect number of type arguments for trait '{}'",
                &trait_decl.human_name.0
            ),
            func.source,
        ));
    }

    let mut values = IndexMap::new();
    for (type_param_name, ty) in trait_decl.params.iter().zip(generic_trait_ref.args.iter()) {
        assert!(values.insert(type_param_name.clone(), ty.clone()).is_none());
    }
    let recipe = PolyRecipe::from(values);

    let return_type = recipe.resolve_type(&trait_func.return_type)?;

    let mut catalog = PolyCatalog::default();
    let params = &trait_func.params;

    for (i, arg) in args.iter().enumerate() {
        let preferred_type =
            (i < params.required.len()).then_some(PreferredType::Reference(&params.required[i].ty));

        let argument_conforms = if let Some(param_type) = preferred_type.map(|p| p.view(ctx.asg)) {
            if param_type.kind.contains_polymorph() {
                let Ok(argument) =
                    conform_expr_to_default::<Perform>(arg, ctx.c_integer_assumptions())
                else {
                    return Err(ResolveError::other(
                        "Cannot conform argument to default value",
                        arg.expr.source,
                    ));
                };

                FuncHaystack::conform_polymorph(ctx, &mut catalog, &argument, param_type)
            } else {
                conform_expr::<Validate>(
                    ctx,
                    &arg,
                    param_type,
                    ConformMode::ParameterPassing,
                    ctx.adept_conform_behavior(),
                    *call_source,
                )
                .is_ok()
            }
        } else {
            conform_expr_to_default::<Validate>(arg, ctx.c_integer_assumptions()).is_ok()
        };

        if !argument_conforms {
            return Err(ResolveError::other(
                if let Some(p) = preferred_type.map(|p| p.view(&ctx.asg)) {
                    format!("Cannot conform argument to expected type '{}'", p)
                } else {
                    format!("Cannot conform argument to default type",)
                },
                arg.expr.source,
            ));
        }
    }

    Ok(TypedExpr::new(
        return_type,
        asg::ExprKind::PolyCall(Box::new(PolyCall {
            callee: PolyCallee {
                polymorph: polymorph.into(),
                member,
            },
            args,
        }))
        .at(*call_source),
    ))
}
