use crate::{
    asg::{self, Callee, GenericTraitRef, ImplParams},
    ast::{self, Using},
    resolve::{
        error::ResolveError,
        expr::{static_member::resolve_impl_mention_from_type, ResolveExprCtx},
        MatchTypesError, PolyCatalog, PolyValue,
    },
    source_files::{source::Sourced, Source},
};
use std::collections::HashSet;

pub fn resolve_impl_arg(
    ctx: &mut ResolveExprCtx,
    callee: &mut Callee,
    using: &Using,
    used_names: &mut HashSet<String>,
    catalog: &mut PolyCatalog,
) -> Result<(), ResolveError> {
    let impl_arg = &using.ty;

    if let ast::TypeKind::Polymorph(polymorph) = &impl_arg.kind {
        resolve_polymorph_impl_arg(ctx, callee, using, polymorph, used_names, catalog)
    } else {
        resolve_concrete_impl_arg(ctx, callee, using, impl_arg, used_names, catalog)
    }
}

fn resolve_concrete_impl_arg(
    ctx: &mut ResolveExprCtx,
    callee: &mut Callee,
    using: &Using,
    impl_arg: &ast::Type,
    used_names: &mut HashSet<String>,
    catalog: &mut PolyCatalog,
) -> Result<(), ResolveError> {
    let impl_arg_source = using.ty.source;
    let (impl_ref, impl_poly_catalog) = resolve_impl_mention_from_type(ctx, impl_arg)?;

    let imp = ctx
        .asg
        .impls
        .get(impl_ref)
        .expect("referenced impl to exist");

    let arg_concrete_trait = impl_poly_catalog.bake().resolve_trait(&imp.target)?;
    let callee_func = ctx.asg.funcs.get(callee.func_ref).unwrap();

    try_register_specified_impl(
        ctx,
        callee_func,
        using,
        impl_arg_source,
        used_names,
        PolyValue::Impl(impl_ref),
        &arg_concrete_trait,
        &callee_func.impl_params,
        catalog,
    )
}

fn resolve_polymorph_impl_arg(
    ctx: &mut ResolveExprCtx,
    callee: &mut Callee,
    using: &Using,
    polymorph: &str,
    used_names: &mut HashSet<String>,
    catalog: &mut PolyCatalog,
) -> Result<(), ResolveError> {
    let impl_arg_source = using.ty.source;
    let callee_func = ctx.asg.funcs.get(callee.func_ref).unwrap();

    let Some(current_func_ref) = ctx.func_ref else {
        return Err(ResolveError::other(
            format!("Undefined implementation polymorph '${}'", polymorph),
            impl_arg_source,
        ));
    };

    let caller = ctx
        .asg
        .funcs
        .get(current_func_ref)
        .expect("referenced function to exist");

    let Some(arg_concrete_trait) = caller.impl_params.get(polymorph) else {
        return Err(ResolveError::other(
            format!("Undefined implementation polymorph '${}'", polymorph),
            impl_arg_source,
        ));
    };

    try_register_specified_impl(
        ctx,
        callee_func,
        using,
        impl_arg_source,
        used_names,
        PolyValue::PolyImpl(polymorph.into()),
        arg_concrete_trait,
        &callee_func.impl_params,
        catalog,
    )
}

fn try_register_specified_impl(
    ctx: &ResolveExprCtx,
    callee_func: &asg::Func,
    using: &Using,
    impl_arg_source: Source,
    used_names: &mut HashSet<String>,
    poly_value: PolyValue,
    arg_concrete_trait: &GenericTraitRef,
    impl_params: &ImplParams,
    catalog: &mut PolyCatalog,
) -> Result<(), ResolveError> {
    let target_param = match &using.name {
        Some(name_and_source) => name_and_source.as_ref(),
        None => Sourced::new(
            callee_func
                .impl_params
                .iter()
                .filter(|(param_name, param)| {
                    param.trait_ref == arg_concrete_trait.trait_ref
                        && !used_names.contains(*param_name)
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
            impl_arg_source,
        ),
    }
    .clone();

    let Some(param_generic_trait) = impl_params.get(target_param.inner().as_str()) else {
        return Err(ResolveError::other(
            format!(
                "No implementation parameter named '${}' exists on callee",
                target_param.inner()
            ),
            target_param.source,
        ));
    };

    if !used_names.insert(target_param.inner().to_string()) {
        return Err(ResolveError::other(
            format!(
                "Implementation for '${}' was already specified",
                target_param.inner()
            ),
            target_param.source,
        ));
    }

    match catalog.extend_if_match_all_types(
        ctx,
        param_generic_trait.args.as_slice(),
        arg_concrete_trait.args.as_slice(),
    ) {
        Ok(()) => {}
        Err(MatchTypesError::LengthMismatch) => {
            return Err(ResolveError::other(
                "Mismatching number of arguments expected for trait implementation",
                target_param.source,
            ));
        }
        Err(MatchTypesError::Incongruent(_) | MatchTypesError::NoMatch(_)) => {
            return Err(ResolveError::other(
                format!(
                    "Implementation of '{}' cannot be used for '{}'",
                    arg_concrete_trait.display(&ctx.asg),
                    param_generic_trait.display(&ctx.asg),
                ),
                impl_arg_source,
            ));
        }
    }

    let param_concrete_trait = catalog.resolver().resolve_trait(param_generic_trait)?;

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

    catalog
        .polymorphs
        .insert(target_param.inner().to_string(), poly_value)
        .is_some()
        .then(|| {
            ResolveError::other(
                format!(
                    "Multiple implementations were specified for implementation parameter '${}'",
                    target_param.inner()
                ),
                target_param.source,
            )
        })
        .map_or(Ok(()), Err)
}
