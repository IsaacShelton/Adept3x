use crate::{
    asg::{Callee, GenericTraitRef},
    ast::{self, Using},
    resolve::{
        error::ResolveError,
        expr::{static_member::resolve_impl_mention_from_type, ResolveExprCtx},
        PolyValue,
    },
    source_files::Source,
};
use std::collections::HashSet;

pub fn resolve_impl_arg(
    ctx: &mut ResolveExprCtx,
    callee: &mut Callee,
    source: Source,
    using: &Using,
    used_names: &mut HashSet<String>,
) -> Result<(), ResolveError> {
    let impl_arg = &using.ty;

    if let ast::TypeKind::Polymorph(polymorph, args_to_polymorph) = &impl_arg.kind {
        resolve_polymorph_impl_arg(ctx, callee, using, polymorph, args_to_polymorph, used_names)
    } else {
        resolve_name_impl_arg(ctx, callee, using, impl_arg, source, used_names)
    }
}

fn resolve_name_impl_arg(
    ctx: &mut ResolveExprCtx,
    callee: &mut Callee,
    using: &Using,
    impl_arg: &ast::Type,
    call_source: Source,
    used_names: &mut HashSet<String>,
) -> Result<(), ResolveError> {
    let impl_arg_source = using.ty.source;
    let (impl_ref, impl_poly_catalog) = resolve_impl_mention_from_type(ctx, impl_arg)?;

    let imp = ctx
        .asg
        .impls
        .get(impl_ref)
        .expect("referenced impl to exist");

    let arg_concrete_trait = impl_poly_catalog.bake().resolve_trait(&imp.target)?;

    let function = ctx.asg.funcs.get(callee.function).unwrap();

    let target_poly_impl_name = match &using.name {
        Some(name) => name,
        None => function
            .impl_params
            .params
            .iter()
            .filter(|(param_name, param)| {
                param.trait_ref == arg_concrete_trait.trait_ref && !used_names.contains(*param_name)
            })
            .map(|(param_name, _)| param_name)
            .next()
            .ok_or_else(|| {
                ResolveError::other(
                    format!(
                        "Excess implementation of trait '{}' is not used by callee",
                        arg_concrete_trait.display(&ctx.asg)
                    ),
                    impl_arg.source,
                )
            })?,
    }
    .clone();

    let Some(param_generic_trait) = function.impl_params.params.get(&target_poly_impl_name) else {
        return Err(ResolveError::other(
            format!(
                "Callee does not have implementation parameter '${}'",
                target_poly_impl_name
            ),
            call_source,
        ));
    };

    try_register_specified_impl(
        ctx,
        callee,
        target_poly_impl_name,
        impl_arg_source,
        used_names,
        PolyValue::Impl(impl_ref),
        &arg_concrete_trait,
        &param_generic_trait,
    )
}

fn resolve_polymorph_impl_arg(
    ctx: &mut ResolveExprCtx,
    callee: &mut Callee,
    using: &Using,
    polymorph: &str,
    args_to_polymorph: &[ast::Type],
    used_names: &mut HashSet<String>,
) -> Result<(), ResolveError> {
    let impl_arg_source = using.ty.source;
    let callee_func = ctx.asg.funcs.get(callee.function).unwrap();

    if !args_to_polymorph.is_empty() {
        return Err(ResolveError::other(
            "Implementation polymorphs cannot take type arguments",
            impl_arg_source,
        ));
    }

    let caller = ctx
        .func_ref
        .and_then(|func_ref| ctx.asg.funcs.get(func_ref))
        .unwrap();

    let Some(arg_concrete_trait) = caller.impl_params.params.get(polymorph) else {
        return Err(ResolveError::other(
            format!("Undefined implementation polymorph '${}'", polymorph),
            impl_arg_source,
        ));
    };

    let target_poly_impl_name = match &using.name {
        Some(name) => name,
        None => callee_func
            .impl_params
            .params
            .iter()
            .filter(|(param_name, param)| {
                param.trait_ref == arg_concrete_trait.trait_ref && !used_names.contains(*param_name)
            })
            .map(|(param_name, _)| param_name)
            .next()
            .ok_or_else(|| {
                ResolveError::other(
                    format!(
                        "Excess implementation of trait '{}' is not used by callee",
                        arg_concrete_trait.display(&ctx.asg)
                    ),
                    impl_arg_source,
                )
            })?,
    }
    .clone();

    let Some(param_generic_trait) = callee_func.impl_params.params.get(&target_poly_impl_name)
    else {
        return Err(ResolveError::other(
            format!(
                "Callee does not have implementation parameter '${}'",
                target_poly_impl_name
            ),
            impl_arg_source,
        ));
    };

    try_register_specified_impl(
        ctx,
        callee,
        target_poly_impl_name,
        impl_arg_source,
        used_names,
        PolyValue::PolyImpl(polymorph.into()),
        arg_concrete_trait,
        param_generic_trait,
    )
}

fn try_register_specified_impl(
    ctx: &ResolveExprCtx,
    callee: &mut Callee,
    target_poly_impl_name: String,
    impl_arg_source: Source,
    used_names: &mut HashSet<String>,
    poly_value: PolyValue,
    arg_concrete_trait: &GenericTraitRef,
    param_generic_trait: &GenericTraitRef,
) -> Result<(), ResolveError> {
    if !used_names.insert(target_poly_impl_name.clone()) {
        return Err(ResolveError::other(
            format!(
                "Implementation for '${}' was already specified",
                target_poly_impl_name
            ),
            impl_arg_source,
        ));
    }

    let param_concrete_trait = callee.recipe.resolve_trait(param_generic_trait)?;

    if *arg_concrete_trait != param_concrete_trait {
        return Err(ResolveError::other(
            format!(
                "Implementation of '{}' cannot be used for '{}'",
                arg_concrete_trait.display(ctx.asg),
                param_concrete_trait.display(ctx.asg)
            ),
            impl_arg_source,
        ));
    }

    callee
        .recipe
        .polymorphs
        .insert(target_poly_impl_name.clone(), poly_value)
        .is_some()
        .then(|| {
            ResolveError::other(
                format!(
                    "Multiple implementations were specified for implementation parameter '${}'",
                    target_poly_impl_name
                ),
                impl_arg_source,
            )
        })
        .map_or(Ok(()), Err)
}
