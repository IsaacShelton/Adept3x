use super::{
    collect_constraints::collect_constraints_into, ctx::ResolveCtx, error::ResolveError,
    func_head::create_func_head, job::FuncJob,
};
use crate::{
    asg::{self, Asg, CurrentConstraints, Func, GenericTraitRef, TraitFunc, Type},
    ast::{self, AstFile},
    cli::BuildOptions,
    name::{Name, ResolvedName},
    resolve::{error::ResolveErrorKind, type_ctx::ResolveTypeCtx},
    source_files::Source,
    workspace::fs::FsNodeId,
};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

pub fn create_impl_heads(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    options: &BuildOptions,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    file: &AstFile,
) -> Result<(), ResolveError> {
    for (impl_i, ast_impl) in file.impls.iter().enumerate() {
        let impl_ref = create_impl_head(ctx, asg, module_file_id, physical_file_id, ast_impl)?;

        for (func_i, func) in ast_impl.body.iter().enumerate() {
            let name = ResolvedName::new(module_file_id, &Name::plain(&func.head.name));

            let func_ref = create_func_head(
                ctx,
                asg,
                options,
                name.clone(),
                &func.head,
                module_file_id,
                physical_file_id,
            )?;

            let asg_impl = asg.impls.get(impl_ref).unwrap();
            let trait_def = asg.traits.get(asg_impl.target.trait_ref).unwrap();
            let concrete_trait = &asg_impl.target;

            let mut expected = HashMap::new();
            for (name, arg) in trait_def.params.iter().zip(concrete_trait.args.iter()) {
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

            let impl_func = asg.funcs.get(func_ref).unwrap();

            ensure_satisfies_trait_func(ctx, asg, expected, trait_func, impl_func)?;

            ctx.jobs
                .push_back(FuncJob::Impling(physical_file_id, impl_i, func_i, func_ref));

            if asg
                .impls
                .get_mut(impl_ref)
                .unwrap()
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

#[derive(Debug, Default)]
struct ForAlls {
    substitution_polys: HashSet<String>,
    trait_to_impl: HashMap<String, String>,
    impl_to_trait: HashMap<String, String>,
}

impl ForAlls {
    pub fn insert(
        &mut self,
        in_trait: String,
        in_impl: String,
        source: Source,
    ) -> Result<(), ResolveError> {
        if self.substitution_polys.contains(&in_impl) {
            return Err(ResolveError::other("Inconsistent mapping", source));
        }

        if let Some(expected) = self.trait_to_impl.get(&in_trait) {
            if *expected != in_impl {
                return Err(ResolveError::other("Inconsistent mapping", source));
            }
        }

        if let Some(expected) = self.impl_to_trait.get(&in_impl) {
            if *expected != in_trait {
                return Err(ResolveError::other("Inconsistent mapping", source));
            }
        }

        if self.trait_to_impl.contains_key(&in_trait) && self.impl_to_trait.contains_key(&in_impl) {
            // Already exists, and is correct
            return Ok(());
        }

        if !self
            .trait_to_impl
            .insert(in_trait.clone(), in_impl.clone())
            .is_none()
            || !self.impl_to_trait.insert(in_impl, in_trait).is_none()
        {
            return Err(ResolveError::other("Inconsistent mapping", source));
        }

        Ok(())
    }
}

fn ensure_satisfies_trait_func(
    ctx: &ResolveCtx,
    asg: &Asg,
    expected: HashMap<&str, &Type>,
    trait_func: &TraitFunc,
    impl_func: &Func,
) -> Result<(), ResolveError> {
    let mut for_alls = ForAlls::default();

    let mut mappings = HashMap::new();
    for sub in expected.values() {
        collect_constraints_into(&mut mappings, sub);
    }
    for_alls.substitution_polys = mappings.into_keys().collect();

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

fn mismatch(source: Source) -> ResolveError {
    ResolveError::other("Type violates expected type required by trait", source)
}

fn matches(
    ctx: &ResolveCtx,
    asg: &Asg,
    expected: &HashMap<&str, &Type>,
    for_alls: &mut ForAlls,
    ty_in_trait: &Type,
    ty_in_impl: &Type,
) -> Result<(), ResolveError> {
    match &ty_in_trait.kind {
        asg::TypeKind::Unresolved => panic!("unresolved"),
        asg::TypeKind::Void
        | asg::TypeKind::Never
        | asg::TypeKind::Boolean
        | asg::TypeKind::Integer(_, _)
        | asg::TypeKind::CInteger(_, _)
        | asg::TypeKind::IntegerLiteral(_)
        | asg::TypeKind::FloatLiteral(_)
        | asg::TypeKind::Floating(_) => (ty_in_impl == ty_in_trait)
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
        | asg::TypeKind::AnonymousEnum() => (ty_in_impl == ty_in_trait)
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
        asg::TypeKind::TypeAlias(_, trait_alias_ref) => match &ty_in_impl.kind {
            asg::TypeKind::TypeAlias(_, impl_alias_ref) => {
                if trait_alias_ref == impl_alias_ref {
                    Ok(())
                } else {
                    Err(mismatch(ty_in_impl.source))
                }
            }
            _ => Err(mismatch(ty_in_impl.source)),
        },
        asg::TypeKind::Polymorph(in_trait, trait_constraints) => {
            if !trait_constraints.is_empty() {
                return Err(mismatch(ty_in_impl.source));
            }

            if let Some(substituation) = expected.get(in_trait.as_str()) {
                if *substituation != ty_in_impl {
                    return Err(mismatch(ty_in_impl.source));
                }
                return Ok(());
            }

            match &ty_in_impl.kind {
                asg::TypeKind::Polymorph(in_impl, impl_constraints) => {
                    if !impl_constraints.is_empty() {
                        return Err(mismatch(ty_in_impl.source));
                    }

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
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    imp: &ast::Impl,
) -> Result<asg::ImplRef, ResolveError> {
    let pre_parameters_constraints = CurrentConstraints::new_empty();

    let type_ctx = ResolveTypeCtx::new(
        &asg,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
        &pre_parameters_constraints,
    );

    if imp
        .params
        .values()
        .any(|param| !param.constraints.is_empty())
    {
        return Err(ResolveError::other(
            "Constraints on implementation name parameters are not supported yet",
            imp.source,
        ));
    }

    let ty = type_ctx.resolve(&imp.target)?;
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

    let abstract_trait = asg
        .traits
        .get(concrete_trait.trait_ref)
        .expect("referenced trait to exist");

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

    let impl_ref = asg.impls.insert(asg::Impl {
        name_params: IndexMap::from_iter(imp.params.keys().cloned().map(|key| (key, ()))),
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

    if ctx
        .impls_in_modules
        .entry(module_file_id)
        .or_default()
        .insert(name.to_string(), impl_ref)
        .is_some()
    {
        return Err(ResolveErrorKind::Other {
            message: format!("Duplicate implementation name '{}'", name),
        }
        .at(imp.source));
    };

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
