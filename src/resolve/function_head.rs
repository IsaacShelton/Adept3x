use super::{
    ctx::ResolveCtx, error::ResolveError, function_haystack::FunctionHaystack, job::FuncJob,
    type_ctx::ResolveTypeCtx,
};
use crate::{
    asg::{self, Asg, Constraint, CurrentConstraints, FuncRef, VariableStorage},
    ast::{self, AstWorkspace, FuncHead},
    cli::BuildOptions,
    hash_map_ext::HashMapExt,
    index_map_ext::IndexMapExt,
    name::{Name, ResolvedName},
    tag::Tag,
    workspace::fs::FsNodeId,
};
use std::collections::{HashMap, HashSet};

fn create_impl_head<'a>(
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

    Ok(asg.impls.insert(asg::Impl {
        ty,
        source: imp.source,
        body: HashMap::default(),
    }))
}

pub fn create_function_heads<'a>(
    ctx: &mut ResolveCtx,
    asg: &mut Asg<'a>,
    ast_workspace: &AstWorkspace,
    options: &BuildOptions,
) -> Result<(), ResolveError> {
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_file_id = ast_workspace.get_owning_module_or_self(*physical_file_id);

        for (impl_i, imp) in file.impls.iter().enumerate() {
            let impl_ref = create_impl_head(ctx, asg, module_file_id, *physical_file_id, imp)?;

            for (function_i, function) in imp.body.iter().enumerate() {
                let name = ResolvedName::new(module_file_id, &Name::plain(&function.head.name));

                let func_ref = create_function_head(
                    ctx,
                    asg,
                    options,
                    name.clone(),
                    &function.head,
                    module_file_id,
                    *physical_file_id,
                )?;

                ctx.jobs.push_back(FuncJob::Impling(
                    *physical_file_id,
                    impl_i,
                    function_i,
                    func_ref,
                ));

                let functions_with_name = asg
                    .impls
                    .get_mut(impl_ref)
                    .unwrap()
                    .body
                    .get_or_insert_with(&function.head.name, || Default::default());

                functions_with_name.push(func_ref);
            }
        }

        for (function_i, function) in file.funcs.iter().enumerate() {
            let name = ResolvedName::new(module_file_id, &Name::plain(&function.head.name));

            let func_ref = create_function_head(
                ctx,
                asg,
                options,
                name.clone(),
                &function.head,
                module_file_id,
                *physical_file_id,
            )?;

            if function.head.privacy.is_public() {
                let function_name = &function.head.name;
                let public_of_module = ctx.public_functions.entry(module_file_id).or_default();

                public_of_module
                    .get_or_insert_with(function_name, || Default::default())
                    .push(func_ref);
            }

            let settings = file.settings.map(|id| &ast_workspace.settings[id.0]);
            let imported_namespaces = settings.map(|settings| &settings.imported_namespaces);

            let function_search_context =
                ctx.function_haystacks
                    .get_or_insert_with(module_file_id, || {
                        FunctionHaystack::new(
                            imported_namespaces
                                .map(|namespaces| namespaces.clone())
                                .unwrap_or_else(|| vec![]),
                            module_file_id,
                        )
                    });

            function_search_context
                .available
                .entry(name)
                .and_modify(|funcs| funcs.push(func_ref))
                .or_insert_with(|| vec![func_ref]);

            ctx.jobs.push_back(FuncJob::Regular(
                *physical_file_id,
                function_i,
                func_ref,
            ));
        }
    }

    Ok(())
}

pub fn create_function_head<'a>(
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
    let parameters = resolve_parameters(&type_ctx, &head.params)?;
    let return_type = type_ctx.resolve(&head.return_type)?;

    let constraints = is_generic
        .then(|| collect_constraints(&parameters, &return_type))
        .unwrap_or_default();

    Ok(asg.funcs.insert(asg::Func {
        name,
        parameters,
        return_type,
        stmts: vec![],
        is_foreign: head.is_foreign,
        variables: VariableStorage::new(),
        source: head.source,
        abide_abi: head.abide_abi,
        tag: head.tag.or_else(|| {
            (options.coerce_main_signature && head.name == "main").then_some(Tag::Main)
        }),
        is_generic,
        constraints: CurrentConstraints::new(constraints),
    }))
}

pub fn resolve_parameters(
    type_ctx: &ResolveTypeCtx,
    parameters: &ast::Params,
) -> Result<asg::Parameters, ResolveError> {
    let mut required = Vec::with_capacity(parameters.required.len());

    for parameter in parameters.required.iter() {
        let ty = type_ctx.resolve(&parameter.ast_type)?;

        required.push(asg::Parameter {
            name: parameter.name.clone(),
            ty,
        });
    }

    Ok(asg::Parameters {
        required,
        is_cstyle_vararg: parameters.is_cstyle_vararg,
    })
}

pub fn collect_constraints(
    parameters: &asg::Parameters,
    return_type: &asg::Type,
) -> HashMap<String, HashSet<Constraint>> {
    let mut map = HashMap::default();

    for param in parameters.required.iter() {
        collect_constraints_into(&mut map, &param.ty);
    }

    collect_constraints_into(&mut map, &return_type);
    map
}

pub fn collect_constraints_into(map: &mut HashMap<String, HashSet<Constraint>>, ty: &asg::Type) {
    match &ty.kind {
        asg::TypeKind::Unresolved => panic!(),
        asg::TypeKind::Boolean
        | asg::TypeKind::Integer(_, _)
        | asg::TypeKind::CInteger(_, _)
        | asg::TypeKind::IntegerLiteral(_)
        | asg::TypeKind::FloatLiteral(_)
        | asg::TypeKind::Floating(_) => (),
        asg::TypeKind::Pointer(inner) => collect_constraints_into(map, inner.as_ref()),
        asg::TypeKind::Void => (),
        asg::TypeKind::AnonymousStruct() => todo!(),
        asg::TypeKind::AnonymousUnion() => todo!(),
        asg::TypeKind::AnonymousEnum() => todo!(),
        asg::TypeKind::FixedArray(fixed_array) => collect_constraints_into(map, &fixed_array.inner),
        asg::TypeKind::FunctionPointer(_) => todo!(),
        asg::TypeKind::Enum(_, _) => (),
        asg::TypeKind::Structure(_, _, parameters) => {
            for parameter in parameters {
                collect_constraints_into(map, parameter);
            }
        }
        asg::TypeKind::TypeAlias(_, _) => (),
        asg::TypeKind::Polymorph(name, constraints) => {
            let set = map.entry(name.to_string()).or_default();
            for constraint in constraints {
                set.insert(constraint.clone());
            }
        }
        asg::TypeKind::Trait(_, _, parameters) => {
            for parameter in parameters {
                collect_constraints_into(map, parameter);
            }
        }
    }
}
