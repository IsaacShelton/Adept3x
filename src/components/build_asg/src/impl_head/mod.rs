mod for_alls;
use super::{
    collect_polymorphs::collect_polymorphs, ctx::ResolveCtx, error::ResolveError,
    func_head::create_func_head, job::FuncJob, type_ctx::ResolveTypeOptions,
};
use crate::{error::ResolveErrorKind, type_ctx::ResolveTypeCtx};
use asg::{Asg, Func, GenericTraitRef, ImplDecl, ImplRef, ResolvedName, TraitFunc, Type};
use ast::Name;
use ast_workspace::NameScope;
use attributes::Privacy;
use compiler::BuildOptions;
use for_alls::ForAlls;
use fs_tree::FsNodeId;
use indexmap::IndexSet;
use source_files::Source;
use std::collections::HashMap;

pub fn create_impl_heads(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    options: &BuildOptions,
    module_folder_id: FsNodeId,
    physical_file_id: FsNodeId,
    ast_file: &NameScope,
) -> Result<(), ResolveError> {
    for impl_id in ast_file.impls.iter() {
        let ast_impl = &asg.workspace.symbols.all_impls[impl_id];
        let impl_ref = create_impl_head(ctx, asg, module_folder_id, physical_file_id, ast_impl)?;

        for (func_i, func) in ast_impl.body.iter().enumerate() {
            let name = ResolvedName::new(module_folder_id, &Name::plain(&func.head.name));

            let func_ref = create_func_head(
                ctx,
                asg,
                options,
                name.clone(),
                &func.head,
                module_folder_id,
                physical_file_id,
            )?;

            let asg_impl = &asg.impls[impl_ref];
            let trait_def = &asg.traits[asg_impl.target.trait_ref];
            let concrete_trait = &asg_impl.target;

            let mut expected = HashMap::new();
            for (name, arg) in trait_def.params.names().zip(concrete_trait.args.iter()) {
                assert!(expected.insert(name.as_str(), arg).is_none());
            }

            let Some(trait_func) = trait_def.funcs.get(&func.head.name) else {
                return Err(ResolveError::other(
                    format!(
                        "Function '{}' is not a member of trait '{}'",
                        &func.head.name, ast_impl.target
                    ),
                    ast_impl.source,
                ));
            };

            let impl_func = &asg.funcs[func_ref];

            ensure_satisfies_trait_func(ctx, asg, expected, trait_func, impl_func)?;

            ctx.jobs.push_back(FuncJob::Impling(
                physical_file_id,
                impl_id,
                func_i,
                func_ref,
            ));

            if asg.impls[impl_ref]
                .body
                .insert(func.head.name.clone(), func_ref)
                .is_some()
            {
                return Err(ResolveError::other(
                    format!("Function '{}' is already implemented", &func.head.name),
                    ast_impl.source,
                ));
            }
        }
    }

    Ok(())
}

fn ensure_satisfies_trait_func(
    ctx: &ResolveCtx,
    asg: &Asg,
    expected: HashMap<&str, &Type>,
    trait_func: &TraitFunc,
    impl_func: &Func,
) -> Result<(), ResolveError> {
    if !impl_func.impl_params.is_empty() {
        return Err(ResolveError::other(
            "Implementation parameter is not allowed by trait definition",
            impl_func.source,
        ));
    }

    let mut mappings = IndexSet::new();
    for sub in expected.values() {
        collect_polymorphs(&mut mappings, sub);
    }

    let mut for_alls = ForAlls::new(mappings);

    if trait_func.params.is_cstyle_vararg != impl_func.params.is_cstyle_vararg {
        return Err(ResolveError::other(
            "Mismatching C variadic-ness",
            impl_func.source,
        ));
    }

    if trait_func.params.required.len() != impl_func.params.required.len() {
        return Err(ResolveError::other(
            if trait_func.params.required.len() == 1 {
                format!(
                    "Expected {} parameter for function '{}'",
                    trait_func.params.required.len(),
                    impl_func.name.name
                )
            } else {
                format!(
                    "Expected {} parameters for function '{}'",
                    trait_func.params.required.len(),
                    impl_func.name.name
                )
            },
            impl_func.source,
        ));
    }

    for (trait_arg, impl_arg) in trait_func
        .params
        .required
        .iter()
        .zip(impl_func.params.required.iter())
    {
        matches(
            ctx,
            asg,
            &expected,
            &mut for_alls,
            &trait_arg.ty,
            &impl_arg.ty,
        )?;
    }

    matches(
        ctx,
        asg,
        &expected,
        &mut for_alls,
        &trait_func.return_type,
        &impl_func.return_type,
    )?;

    Ok(())
}

fn matches(
    ctx: &ResolveCtx,
    asg: &Asg,
    expected: &HashMap<&str, &Type>,
    for_alls: &mut ForAlls,
    ty_in_trait: &Type,
    ty_in_impl: &Type,
) -> Result<(), ResolveError> {
    let mismatch = |source| {
        ResolveError::other(
            format!(
                "Type '{}' violates expected type required by trait '{}'",
                ty_in_impl.to_string(),
                ty_in_trait.to_string(),
            ),
            source,
        )
    };

    match &ty_in_trait.kind {
        asg::TypeKind::Unresolved => panic!("unresolved"),
        asg::TypeKind::Void
        | asg::TypeKind::Never
        | asg::TypeKind::Boolean
        | asg::TypeKind::Integer(_, _)
        | asg::TypeKind::CInteger(_, _)
        | asg::TypeKind::SizeInteger(_)
        | asg::TypeKind::IntegerLiteral(_)
        | asg::TypeKind::FloatLiteral(_)
        | asg::TypeKind::Floating(_) => (ty_in_impl.kind == ty_in_trait.kind)
            .then_some(Ok(()))
            .unwrap_or_else(|| Err(mismatch(ty_in_impl.source))),
        asg::TypeKind::Ptr(trait_inner) => match &ty_in_impl.kind {
            asg::TypeKind::Ptr(impl_inner) => {
                matches(ctx, asg, expected, for_alls, trait_inner, impl_inner)
            }
            _ => Err(mismatch(ty_in_impl.source)),
        },
        asg::TypeKind::AnonymousStruct()
        | asg::TypeKind::AnonymousUnion()
        | asg::TypeKind::AnonymousEnum(_) => (ty_in_impl == ty_in_trait)
            .then_some(Ok(()))
            .unwrap_or_else(|| Err(mismatch(ty_in_impl.source))),
        asg::TypeKind::FixedArray(trait_fixed_array) => match &ty_in_impl.kind {
            asg::TypeKind::FixedArray(impl_fixed_array) => {
                if trait_fixed_array.size != impl_fixed_array.size {
                    return Err(mismatch(ty_in_impl.source));
                }

                matches(
                    ctx,
                    asg,
                    expected,
                    for_alls,
                    &trait_fixed_array.inner,
                    &impl_fixed_array.inner,
                )
            }
            _ => Err(mismatch(ty_in_impl.source)),
        },
        asg::TypeKind::FuncPtr(trait_func_ptr) => match &ty_in_impl.kind {
            asg::TypeKind::FuncPtr(impl_func_ptr) => {
                if trait_func_ptr.params.required.len() != impl_func_ptr.params.required.len() {
                    return Err(mismatch(ty_in_impl.source));
                }

                for (trait_arg, impl_arg) in trait_func_ptr
                    .params
                    .required
                    .iter()
                    .zip(impl_func_ptr.params.required.iter())
                {
                    matches(ctx, asg, expected, for_alls, &trait_arg.ty, &impl_arg.ty)?;
                }

                if trait_func_ptr.params.is_cstyle_vararg != impl_func_ptr.params.is_cstyle_vararg {
                    return Err(mismatch(ty_in_impl.source));
                }

                Ok(())
            }
            _ => Err(mismatch(ty_in_impl.source)),
        },
        asg::TypeKind::Enum(_, trait_enum_ref) => match &ty_in_impl.kind {
            asg::TypeKind::Enum(_, impl_enum_ref) => {
                if trait_enum_ref == impl_enum_ref {
                    Ok(())
                } else {
                    return Err(mismatch(ty_in_impl.source));
                }
            }
            _ => Err(mismatch(ty_in_impl.source)),
        },
        asg::TypeKind::Structure(_, trait_struct_ref, trait_args) => match &ty_in_impl.kind {
            asg::TypeKind::Structure(_, impl_struct_ref, impl_args) => {
                if trait_struct_ref != impl_struct_ref || trait_args.len() != impl_args.len() {
                    return Err(mismatch(ty_in_impl.source));
                }

                for (trait_arg, impl_arg) in trait_args.iter().zip(impl_args.iter()) {
                    matches(ctx, asg, expected, for_alls, trait_arg, impl_arg)?;
                }

                Ok(())
            }
            _ => Err(mismatch(ty_in_impl.source)),
        },
        asg::TypeKind::TypeAlias(_, trait_type_alias_ref, trait_args) => match &ty_in_impl.kind {
            asg::TypeKind::TypeAlias(_, impl_type_alias_ref, impl_args) => {
                if trait_type_alias_ref != impl_type_alias_ref
                    || trait_args.len() != impl_args.len()
                {
                    return Err(mismatch(ty_in_impl.source));
                }

                for (trait_arg, impl_arg) in trait_args.iter().zip(impl_args.iter()) {
                    matches(ctx, asg, expected, for_alls, trait_arg, impl_arg)?;
                }

                Ok(())
            }
            _ => Err(mismatch(ty_in_impl.source)),
        },
        asg::TypeKind::Polymorph(in_trait) => {
            if let Some(substitution) = expected.get(in_trait.as_str()) {
                if *substitution != ty_in_impl {
                    return Err(mismatch(ty_in_impl.source));
                }
                return Ok(());
            }

            match &ty_in_impl.kind {
                asg::TypeKind::Polymorph(in_impl) => {
                    for_alls.insert(in_trait.clone(), in_impl.clone(), ty_in_impl.source)
                }
                _ => Err(mismatch(ty_in_impl.source)),
            }
        }
        asg::TypeKind::Trait(_, _, _) => Err(mismatch(ty_in_impl.source)),
    }
}

pub fn create_impl_head<'a>(
    ctx: &mut ResolveCtx,
    asg: &mut Asg<'a>,
    module_fs_node_id: FsNodeId,
    physical_fs_node_id: FsNodeId,
    imp: &ast::Impl,
) -> Result<asg::ImplRef, ResolveError> {
    let type_ctx = ResolveTypeCtx::new(
        &asg,
        module_fs_node_id,
        physical_fs_node_id,
        &ctx.types_in_modules,
    );

    let ty = type_ctx.resolve(&imp.target, ResolveTypeOptions::Unalias)?;
    let concrete_trait = into_trait(&ty)?;
    let mut func_names = HashMap::new();

    for (index, ast_func) in imp.body.iter().enumerate() {
        if func_names.insert(&ast_func.head.name, index).is_some() {
            return Err(ResolveError::other(
                format!(
                    "Function '{}' cannot have multiple definitions",
                    &ast_func.head.name
                ),
                ast_func.head.source,
            ));
        }
    }

    let abstract_trait = &asg.traits[concrete_trait.trait_ref];

    if concrete_trait.args.len() != abstract_trait.params.len() {
        return Err(ResolveError::other(
            format!("Wrong number of type arguments for trait"),
            ty.source,
        ));
    }

    for (func_name, trait_func) in abstract_trait.funcs.iter() {
        let Some(index) = func_names.get(func_name) else {
            return Err(ResolveError::other(
                format!("Missing function '{}' to satisfy trait", func_name),
                imp.source,
            ));
        };

        let ast_func = imp.body.get(*index).unwrap();

        // Ensure whether using C variadic arguments matches
        if ast_func.head.params.is_cstyle_vararg != trait_func.params.is_cstyle_vararg {
            return Err(ResolveError::other(
                format!(
                    "Mismatching C variadic-ness for function '{}'",
                    ast_func.head.name
                ),
                imp.source,
            ));
        }
    }

    let impl_ref = asg.impls.alloc(asg::Impl {
        params: imp.params.clone(),
        target: concrete_trait,
        source: imp.source,
        body: HashMap::default(),
    });

    if imp.name.is_none() {
        return Err(ResolveError::other(
            "Unnamed trait implementations are not supported yet",
            imp.source,
        ));
    }

    let name = imp
        .name
        .as_ref()
        .map_or("<unnamed impl>", |name| name.as_str());

    declare_impl(
        ctx,
        module_fs_node_id,
        name,
        imp.source,
        imp.privacy,
        impl_ref,
    )?;

    Ok(impl_ref)
}

fn into_trait(ty: &Type) -> Result<GenericTraitRef, ResolveError> {
    let asg::TypeKind::Trait(_, trait_ref, args) = &ty.kind else {
        return Err(ResolveErrorKind::TypeIsNotATrait {
            name: ty.to_string(),
        }
        .at(ty.source));
    };

    Ok(GenericTraitRef {
        trait_ref: *trait_ref,
        args: args.clone(),
    })
}

fn declare_impl(
    ctx: &mut ResolveCtx,
    module_fs_node_id: FsNodeId,
    name: &str,
    source: Source,
    privacy: Privacy,
    impl_ref: ImplRef,
) -> Result<(), ResolveError> {
    if ctx
        .impls_in_modules
        .entry(module_fs_node_id)
        .or_default()
        .insert(
            name.to_string(),
            ImplDecl {
                impl_ref,
                privacy,
                source,
            },
        )
        .is_some()
    {
        return Err(ResolveErrorKind::DuplicateImplementationName {
            name: name.to_string(),
        }
        .at(source));
    };

    Ok(())
}
