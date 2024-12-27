use super::{
    ctx::ResolveCtx, error::ResolveError, function_haystack::FunctionHaystack, job::FuncJob,
    type_ctx::ResolveTypeCtx,
};
use crate::{
    ast::{self, AstWorkspace, FunctionHead},
    cli::BuildOptions,
    hash_map_ext::HashMapExt,
    index_map_ext::IndexMapExt,
    name::{Name, ResolvedName},
    resolved::{self, Constraint, CurrentConstraints, FunctionRef, VariableStorage},
    tag::Tag,
    workspace::fs::FsNodeId,
};
use std::collections::{HashMap, HashSet};

fn create_impl_head<'a>(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast<'a>,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    imp: &ast::Impl,
) -> Result<resolved::ImplRef, ResolveError> {
    let pre_parameters_constraints = CurrentConstraints::new_empty();

    let type_ctx = ResolveTypeCtx::new(
        &resolved_ast,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
        &pre_parameters_constraints,
    );

    // NOTE: This will need to be resolved to which trait to use instead of an actual type
    let resolved_type = type_ctx.resolve(&imp.target)?;

    Ok(resolved_ast.impls.insert(resolved::Impl {
        resolved_type,
        source: imp.source,
        body: vec![],
    }))
}

pub fn create_function_heads<'a>(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast<'a>,
    ast_workspace: &AstWorkspace,
    options: &BuildOptions,
) -> Result<(), ResolveError> {
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_file_id = ast_workspace.get_owning_module_or_self(*physical_file_id);

        for (impl_i, imp) in file.impls.iter().enumerate() {
            let impl_ref =
                create_impl_head(ctx, resolved_ast, module_file_id, *physical_file_id, imp)?;

            for (function_i, function) in imp.body.iter().enumerate() {
                let name = ResolvedName::new(module_file_id, &Name::plain(&function.head.name));

                let function_ref = create_function_head(
                    ctx,
                    resolved_ast,
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
                    function_ref,
                ));
            }
        }

        for (function_i, function) in file.functions.iter().enumerate() {
            let name = ResolvedName::new(module_file_id, &Name::plain(&function.head.name));

            let function_ref = create_function_head(
                ctx,
                resolved_ast,
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
                    .push(function_ref);
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
                .and_modify(|funcs| funcs.push(function_ref))
                .or_insert_with(|| vec![function_ref]);

            ctx.jobs.push_back(FuncJob::Regular(
                *physical_file_id,
                function_i,
                function_ref,
            ));
        }
    }

    Ok(())
}

pub fn create_function_head<'a>(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast<'a>,
    options: &BuildOptions,
    name: ResolvedName,
    head: &FunctionHead,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
) -> Result<FunctionRef, ResolveError> {
    let pre_parameters_constraints = CurrentConstraints::new_empty();

    let type_ctx = ResolveTypeCtx::new(
        &resolved_ast,
        module_file_id,
        physical_file_id,
        &ctx.types_in_modules,
        &pre_parameters_constraints,
    );

    let is_generic = head.is_generic();
    let parameters = resolve_parameters(&type_ctx, &head.parameters)?;
    let return_type = type_ctx.resolve(&head.return_type)?;

    let constraints = is_generic
        .then(|| collect_constraints(&parameters, &return_type))
        .unwrap_or_default();

    Ok(resolved_ast.functions.insert(resolved::Function {
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
    parameters: &ast::Parameters,
) -> Result<resolved::Parameters, ResolveError> {
    let mut required = Vec::with_capacity(parameters.required.len());

    for parameter in parameters.required.iter() {
        let resolved_type = type_ctx.resolve(&parameter.ast_type)?;

        required.push(resolved::Parameter {
            name: parameter.name.clone(),
            resolved_type,
        });
    }

    Ok(resolved::Parameters {
        required,
        is_cstyle_vararg: parameters.is_cstyle_vararg,
    })
}

pub fn collect_constraints(
    parameters: &resolved::Parameters,
    return_type: &resolved::Type,
) -> HashMap<String, HashSet<Constraint>> {
    let mut map = HashMap::default();

    for param in parameters.required.iter() {
        collect_constraints_into(&mut map, &param.resolved_type);
    }

    collect_constraints_into(&mut map, &return_type);
    map
}

pub fn collect_constraints_into(
    map: &mut HashMap<String, HashSet<Constraint>>,
    ty: &resolved::Type,
) {
    match &ty.kind {
        resolved::TypeKind::Unresolved => panic!(),
        resolved::TypeKind::Boolean
        | resolved::TypeKind::Integer(_, _)
        | resolved::TypeKind::CInteger(_, _)
        | resolved::TypeKind::IntegerLiteral(_)
        | resolved::TypeKind::FloatLiteral(_)
        | resolved::TypeKind::Floating(_) => (),
        resolved::TypeKind::Pointer(inner) => collect_constraints_into(map, inner.as_ref()),
        resolved::TypeKind::Void => (),
        resolved::TypeKind::AnonymousStruct() => todo!(),
        resolved::TypeKind::AnonymousUnion() => todo!(),
        resolved::TypeKind::AnonymousEnum() => todo!(),
        resolved::TypeKind::FixedArray(fixed_array) => {
            collect_constraints_into(map, &fixed_array.inner)
        }
        resolved::TypeKind::FunctionPointer(_) => todo!(),
        resolved::TypeKind::Enum(_, _) => (),
        resolved::TypeKind::Structure(_, _, parameters) => {
            for parameter in parameters {
                collect_constraints_into(map, parameter);
            }
        }
        resolved::TypeKind::TypeAlias(_, _) => (),
        resolved::TypeKind::Polymorph(name, constraints) => {
            let set = map.entry(name.to_string()).or_default();
            for constraint in constraints {
                set.insert(constraint.clone());
            }
        }
        resolved::TypeKind::Trait(_, _) => (),
    }
}
