use super::{call::call_callee, resolve_expr, ResolveExprCtx, ResolveExprMode};
use crate::{
    asg::{self, HumanName, ImplRef, PolyCall, PolyCallee, TypeKind, TypedExpr},
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
        resolve_type_args_to_poly_args,
        type_ctx::ResolveTypeOptions,
        PolyCatalog, PolyRecipe,
    },
    source_files::Source,
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

    let ty = ctx
        .type_ctx()
        .resolve(&subject, ResolveTypeOptions::Unalias)?;

    let extracted = match &ty.kind {
        TypeKind::AnonymousEnum(enumeration) => enumeration.members.get(value).map(|member| {
            (
                HumanName("(anonymous enum)".into()),
                asg::EnumTarget::Anonymous(member.value.clone(), ty.clone()),
            )
        }),
        TypeKind::Enum(human_name, enum_ref) => {
            Some((human_name.clone(), asg::EnumTarget::Named(*enum_ref)))
        }
        _ => {
            return Err(ResolveError::other(
                format!("Type '{}' is not an enum", subject),
                *value_source,
            ));
        }
    };

    let Some((human_name, enum_target)) = extracted else {
        return Err(ResolveErrorKind::StaticMemberOfTypeDoesNotExist {
            ty: subject.to_string(),
            member: value.to_string(),
        }
        .at(*value_source));
    };

    Ok(TypedExpr::new(
        ty,
        asg::Expr::new(
            asg::ExprKind::EnumMemberLiteral(Box::new(asg::EnumMemberLiteral {
                human_name,
                enum_target,
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
        ast::TypeKind::Polymorph(polymorph) => {
            resolve_static_member_call_polymorph(ctx, static_member_call, polymorph)
        }
        _ => Err(ResolveError::other(
            "Using callee supplied trait implementations is not supported yet",
            static_member_call.source,
        )),
    }
}

pub fn resolve_impl_mention_from_type<'a>(
    ctx: &mut ResolveExprCtx,
    ty: &ast::Type,
) -> Result<(ImplRef, PolyCatalog), ResolveError> {
    let ast::TypeKind::Named(name, type_args) = &ty.kind else {
        return Err(ResolveError::other(
            "Expected implementation name",
            ty.source,
        ));
    };

    resolve_impl_mention(ctx, name, type_args, ty.source)
}

pub fn resolve_impl_mention(
    ctx: &mut ResolveExprCtx,
    impl_name: &Name,
    impl_args: &[TypeArg],
    source: Source,
) -> Result<(ImplRef, PolyCatalog), ResolveError> {
    let impl_decl = impl_name.as_plain_str().and_then(|impl_name| {
        ctx.impls_in_modules
            .get(&ctx.module_fs_node_id)
            .and_then(|impls| impls.get(impl_name))
    });

    let impl_decl = impl_decl.ok_or(()).or_else(|_| {
        let mut matches = (!impl_name.namespace.is_empty())
            .then(|| {
                ctx.settings
                    .namespace_to_dependency
                    .get(impl_name.namespace.as_ref())
            })
            .flatten()
            .into_iter()
            .flatten()
            .flat_map(|dependency| {
                ctx.settings
                    .dependency_to_module
                    .get(dependency)
                    .and_then(|module_fs_node_id| ctx.impls_in_modules.get(module_fs_node_id))
                    .and_then(|imp| imp.get(impl_name.basename.as_ref()))
                    .filter(|imp| imp.privacy.is_public())
                    .into_iter()
            });

        let Some(imp) = matches.next() else {
            return Err(ResolveError::other(
                format!("Undefined trait implementation '{}'", impl_name),
                source,
            ));
        };

        if matches.next().is_some() {
            return Err(ResolveError::other(
                format!("Ambiguous trait implementation '{}'", impl_name),
                source,
            ));
        }

        Ok(imp)
    })?;

    let imp = ctx
        .asg
        .impls
        .get(impl_decl.impl_ref)
        .expect("public impl of impl decl to exist");

    if imp.params.len() != impl_args.len() {
        return Err(ResolveError::other(
            "Wrong number of arguments for implementation",
            source,
        ));
    }

    let mut catalog = PolyCatalog::default();

    for (name, arg) in imp.params.names().zip(impl_args) {
        match arg {
            ast::TypeArg::Type(ty) => {
                catalog
                    .put_type(
                        name,
                        &ctx.type_ctx().resolve(ty, ResolveTypeOptions::Unalias)?,
                    )
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

    Ok((impl_decl.impl_ref, catalog))
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
        source: _,
    } = &static_member_call;

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

    let (impl_ref, catalog) =
        resolve_impl_mention(ctx, impl_name, impl_args, static_member_call.source)?;

    let Some(callee_name) = call.name.as_plain_str() else {
        return Err(ResolveError::other(
            "Implementation does not have namespaced functions",
            *call_source,
        ));
    };

    let generics = resolve_type_args_to_poly_args(ctx, &static_member_call.call.generics)?;

    let imp = ctx
        .asg
        .impls
        .get(impl_ref)
        .expect("referenced impl to exist");

    let mut only_match = imp
        .body
        .get(callee_name)
        .into_iter()
        .copied()
        .flat_map(|func_ref| {
            FuncHaystack::fits(
                ctx,
                func_ref,
                &generics,
                &args,
                Some(catalog.clone()),
                *call_source,
            )
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

    let Some(generic_trait_ref) = func.impl_params.get(polymorph) else {
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
    for (type_param_name, ty) in trait_decl.params.names().zip(generic_trait_ref.args.iter()) {
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
                recipe: catalog.bake(),
            },
            args,
        }))
        .at(*call_source),
    ))
}
