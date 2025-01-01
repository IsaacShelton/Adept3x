use super::{ctx::ResolveCtx, error::ResolveError, func_head::create_func_head, job::FuncJob};
use crate::{
    asg::{self, Asg, CurrentConstraints, GenericTraitRef, Type},
    ast::{self, AstFile},
    cli::BuildOptions,
    hash_map_ext::HashMapExt,
    name::{Name, ResolvedName},
    resolve::{error::ResolveErrorKind, type_ctx::ResolveTypeCtx},
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
    for (impl_i, imp) in file.impls.iter().enumerate() {
        let impl_ref = create_impl_head(ctx, asg, module_file_id, physical_file_id, imp)?;

        for (func_i, func) in imp.body.iter().enumerate() {
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

            ctx.jobs
                .push_back(FuncJob::Impling(physical_file_id, impl_i, func_i, func_ref));

            asg.impls
                .get_mut(impl_ref)
                .unwrap()
                .body
                .get_or_insert_with(&func.head.name, || Default::default())
                .push(func_ref);
        }
    }

    Ok(())
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

    // NOTE: This will need to be resolved to which trait to use instead of an actual type
    let ty = type_ctx.resolve(&imp.target)?;

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

    let target = into_trait(&ty)?;
    let mut func_names = HashSet::new();

    for ast_func in imp.body.iter() {
        if !func_names.insert(&ast_func.head.name) {
            return Err(ResolveError::other(
                format!(
                    "Function '{}' cannot have multiple definitions",
                    &ast_func.head.name
                ),
                ast_func.head.source,
            ));
        }
    }

    for trait_func in asg
        .traits
        .get(target.trait_ref)
        .expect("referenced trait to exist")
        .funcs
        .iter()
    {
        if !func_names.contains(&trait_func.name) {
            return Err(ResolveError::other(
                format!("Missing function '{}' to satisfy trait", &trait_func.name,),
                imp.source,
            ));
        }
    }

    let impl_ref = asg.impls.insert(asg::Impl {
        name_params: IndexMap::from_iter(imp.params.keys().cloned().map(|key| (key, ()))),
        target,
        source: imp.source,
        body: HashMap::default(),
    });

    // TODO: Check that all methods of trait being implemented are satisfied
    eprintln!("warning: whether implementation implements trait is unchecked yet!");

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
