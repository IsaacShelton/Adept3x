use super::{
    collect_constraints::collect_constraints, ctx::ResolveCtx, error::ResolveError,
    func_haystack::FuncHaystack, impl_head::create_impl_heads, job::FuncJob,
    type_ctx::ResolveTypeCtx,
};
use crate::{
    asg::{self, Asg, CurrentConstraints, FuncRef, GenericTraitRef, ImplParams, VariableStorage},
    ast::{self, AstWorkspace, FuncHead},
    cli::BuildOptions,
    hash_map_ext::HashMapExt,
    index_map_ext::IndexMapExt,
    name::{Name, ResolvedName},
    tag::Tag,
    workspace::fs::FsNodeId,
};
use indexmap::IndexMap;
use std::borrow::Cow;

pub fn create_func_heads<'a>(
    ctx: &mut ResolveCtx,
    asg: &mut Asg<'a>,
    ast_workspace: &AstWorkspace,
    options: &BuildOptions,
) -> Result<(), ResolveError> {
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_file_id = ast_workspace.get_owning_module_or_self(*physical_file_id);

        create_impl_heads(ctx, asg, options, module_file_id, *physical_file_id, file)?;

        for (func_i, func) in file.funcs.iter().enumerate() {
            let name = ResolvedName::new(module_file_id, &Name::plain(&func.head.name));

            let func_ref = create_func_head(
                ctx,
                asg,
                options,
                name.clone(),
                &func.head,
                module_file_id,
                *physical_file_id,
            )?;

            if func.head.privacy.is_public() {
                let name = &func.head.name;
                let public_of_module = ctx.public_funcs.entry(module_file_id).or_default();

                public_of_module
                    .get_or_insert_with(name, || Default::default())
                    .push(func_ref);
            }

            let settings = file.settings.map(|id| &ast_workspace.settings[id.0]);
            let imported_namespaces = settings.map(|settings| &settings.imported_namespaces);

            let func_haystack = ctx.func_haystacks.get_or_insert_with(module_file_id, || {
                FuncHaystack::new(
                    imported_namespaces
                        .map(|namespaces| namespaces.clone())
                        .unwrap_or_else(|| vec![]),
                    module_file_id,
                )
            });

            func_haystack
                .available
                .entry(name)
                .and_modify(|funcs| funcs.push(func_ref))
                .or_insert_with(|| vec![func_ref]);

            ctx.jobs
                .push_back(FuncJob::Regular(*physical_file_id, func_i, func_ref));
        }
    }

    Ok(())
}

pub fn create_func_head<'a>(
    ctx: &mut ResolveCtx,
    asg: &mut Asg<'a>,
    options: &BuildOptions,
    name: ResolvedName,
    head: &FuncHead,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
) -> Result<FuncRef, ResolveError> {
    let pre_parameters_constraints = CurrentConstraints::new_empty();

    let type_ctx = ResolveTypeCtx::new(
        &asg,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
        &pre_parameters_constraints,
    );

    let is_generic = head.is_generic();
    let params = resolve_parameters(&type_ctx, &head.params)?;
    let return_type = type_ctx.resolve(&head.return_type)?;

    let constraints = is_generic
        .then(|| collect_constraints(&params, &return_type))
        .unwrap_or_default();

    let impl_params = create_func_impl_params(&type_ctx, head)?;

    let is_main = options.coerce_main_signature && head.name == "main";

    if is_main && impl_params.has_items() {
        return Err(ResolveError::other(
            "Main function cannot have implementation parameters",
            head.source,
        ));
    }

    Ok(asg.funcs.insert(asg::Func {
        name,
        params,
        return_type,
        stmts: vec![],
        is_foreign: head.is_foreign,
        vars: VariableStorage::new(),
        source: head.source,
        abide_abi: head.abide_abi,
        tag: head.tag.or_else(|| is_main.then_some(Tag::Main)),
        is_generic,
        constraints: CurrentConstraints::new(constraints),
        impl_params,
    }))
}

pub fn resolve_parameters(
    type_ctx: &ResolveTypeCtx,
    parameters: &ast::Params,
) -> Result<asg::Params, ResolveError> {
    let mut required = Vec::with_capacity(parameters.required.len());

    for parameter in parameters.required.iter() {
        let ty = type_ctx.resolve(&parameter.ast_type)?;

        required.push(asg::Param {
            name: parameter.name.clone(),
            ty,
        });
    }

    Ok(asg::Params {
        required,
        is_cstyle_vararg: parameters.is_cstyle_vararg,
    })
}

pub fn create_func_impl_params(
    type_ctx: &ResolveTypeCtx,
    head: &FuncHead,
) -> Result<ImplParams, ResolveError> {
    let mut params = IndexMap::default();

    for (i, given) in head.givens.iter().enumerate() {
        let trait_ty = type_ctx.resolve(&given.ty)?;

        let asg::TypeKind::Trait(_, trait_ref, trait_args) = &trait_ty.kind else {
            return Err(ResolveError::other("Expected trait", trait_ty.source));
        };

        let generic_trait_ref = GenericTraitRef {
            trait_ref: *trait_ref,
            args: trait_args.to_vec(),
        };

        let (name, name_source) = given
            .name
            .as_ref()
            .map(|(name, name_source)| (Cow::Borrowed(name), *name_source))
            .unwrap_or_else(|| (Cow::Owned(format!(".{}", i)), given.ty.source));

        if params
            .insert(name.as_ref().clone(), generic_trait_ref)
            .is_some()
        {
            return Err(ResolveError::other(
                format!("Trait implementation polymorph '${}' already exists", name),
                name_source,
            ));
        }
    }

    Ok(ImplParams::new(params))
}
