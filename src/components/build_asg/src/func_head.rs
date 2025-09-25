use super::{
    ctx::ResolveCtx,
    error::ResolveError,
    func_haystack::FuncHaystack,
    impl_head::create_impl_heads,
    job::FuncJob,
    type_ctx::{ResolveTypeCtx, ResolveTypeOptions},
};
use asg::{Asg, FuncRef, GenericTraitRef, ImplParams, ResolvedName, VariableStorage};
use ast::{FuncHead, NamePath};
use ast_workspace::AstWorkspace;
use attributes::{Exposure, SymbolOwnership, Tag};
use compiler::BuildOptions;
use fs_tree::FsNodeId;
use indexmap::IndexMap;
use source_files::Sourced;
use std::borrow::Cow;
use std_ext::{HashMapExt, IndexMapExt};

pub fn create_func_heads<'a>(
    ctx: &mut ResolveCtx,
    asg: &mut Asg<'a>,
    ast_workspace: &AstWorkspace,
    options: &BuildOptions,
) -> Result<(), ResolveError> {
    for (physical_file_id, ast_file) in ast_workspace.files.iter() {
        let module_folder_id = ast_workspace.get_owning_module_or_self(physical_file_id);

        create_impl_heads(
            ctx,
            asg,
            options,
            module_folder_id,
            physical_file_id,
            &ast_workspace.symbols.all_name_scopes[ast_file.names],
        )?;

        for func_id in ast_workspace.symbols.all_name_scopes[ast_file.names]
            .funcs
            .iter()
        {
            let func = &ast_workspace.symbols.all_funcs[func_id];

            let name = if func.head.privacy.is_private() {
                ResolvedName::new(
                    physical_file_id,
                    &NamePath::new_plain(func.head.name.clone()),
                )
            } else {
                ResolvedName::new(
                    module_folder_id,
                    &NamePath::new_plain(func.head.name.clone()),
                )
            };

            let func_ref = create_func_head(
                ctx,
                asg,
                options,
                name.clone(),
                &func.head,
                module_folder_id,
                physical_file_id,
            )?;

            if func.head.privacy.is_public() {
                let name = &func.head.name;
                let public_of_module = ctx.public_funcs.entry(module_folder_id).or_default();

                public_of_module
                    .get_or_insert_with(name, || Default::default())
                    .push(func_ref);
            }

            let imported_namespaces = &ast_workspace.view(ast_file).settings.imported_namespaces;

            let func_haystack = ctx.func_haystacks.get_or_insert_with(module_folder_id, || {
                FuncHaystack::new(imported_namespaces.clone(), module_folder_id)
            });

            func_haystack
                .available
                .entry(name)
                .and_modify(|funcs| funcs.push(func_ref))
                .or_insert_with(|| vec![func_ref]);

            ctx.jobs
                .push_back(FuncJob::Regular(physical_file_id, func_id, func_ref));
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
    module_folder_id: FsNodeId,
    physical_file_id: FsNodeId,
) -> Result<FuncRef, ResolveError> {
    let type_ctx = ResolveTypeCtx::new(
        &asg,
        module_folder_id,
        physical_file_id,
        &ctx.types_in_modules,
    );

    let is_generic = head.is_generic();
    let params = resolve_parameters(&type_ctx, &head.params)?;
    let return_type = type_ctx.resolve(&head.return_type, ResolveTypeOptions::Unalias)?;
    let impl_params = create_func_impl_params(&type_ctx, head)?;
    let is_main = options.coerce_main_signature && head.tag == Some(Tag::Main);

    if is_main && !impl_params.is_empty() {
        return Err(ResolveError::other(
            "Main function cannot have implementation parameters",
            head.source,
        ));
    }

    Ok(asg.funcs.alloc(asg::Func {
        name,
        type_params: head.type_params.clone(),
        params,
        return_type,
        stmts: vec![],
        vars: VariableStorage::new(),
        source: head.source,
        abide_abi: head.abide_abi,
        tag: head.tag.or_else(|| is_main.then_some(Tag::Main)),
        ownership: if is_main {
            SymbolOwnership::Owned(Exposure::Exposed)
        } else {
            head.ownership
        },
        is_generic,
        impl_params,
    }))
}

pub fn resolve_parameters(
    type_ctx: &ResolveTypeCtx,
    parameters: &ast::Params,
) -> Result<asg::Params, ResolveError> {
    let mut required = Vec::with_capacity(parameters.required.len());

    for parameter in parameters.required.iter() {
        let ty = type_ctx.resolve(&parameter.ast_type, ResolveTypeOptions::Unalias)?;

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
        let trait_ty = type_ctx.resolve(&given.ty, ResolveTypeOptions::Unalias)?;

        let asg::TypeKind::Trait(_, trait_ref, trait_args) = &trait_ty.kind else {
            return Err(ResolveError::other("Expected trait", trait_ty.source));
        };

        let generic_trait_ref = GenericTraitRef {
            trait_ref: *trait_ref,
            args: trait_args.to_vec(),
        };

        let sourced_name = given
            .name
            .as_ref()
            .map(|sourced_name| {
                Sourced::new(
                    Cow::Borrowed(sourced_name.inner().as_str()),
                    sourced_name.source,
                )
            })
            .unwrap_or_else(|| Sourced::new(Cow::Owned(format!(".{}", i)), given.ty.source));

        if params
            .insert(sourced_name.inner().as_ref().to_string(), generic_trait_ref)
            .is_some()
        {
            return Err(ResolveError::other(
                format!(
                    "Trait implementation polymorph '${}' already exists",
                    sourced_name.inner()
                ),
                sourced_name.source,
            ));
        }
    }

    Ok(ImplParams::new(params))
}
