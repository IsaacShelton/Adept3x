use super::{
    ctx::ResolveCtx, error::ResolveError, function_haystack::FunctionHaystack, job::FuncJob,
    type_ctx::ResolveTypeCtx,
};
use crate::{
    ast::{self, AstWorkspace},
    cli::BuildOptions,
    index_map_ext::IndexMapExt,
    name::ResolvedName,
    resolved::{self, Constraint, VariableStorage},
    tag::Tag,
};
use std::collections::HashSet;

pub fn create_function_heads(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    ast_workspace: &AstWorkspace,
    options: &BuildOptions,
) -> Result<(), ResolveError> {
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_file_id = ast_workspace
            .get_owning_module(*physical_file_id)
            .unwrap_or(*physical_file_id);

        for (function_i, function) in file.functions.iter().enumerate() {
            let name = ResolvedName::new(module_file_id, &function.name);
            let type_ctx = ResolveTypeCtx::new(
                &resolved_ast,
                module_file_id,
                *physical_file_id,
                &ctx.types_in_modules,
            );

            let is_generic = function.return_type.contains_polymorph().is_some()
                || function
                    .parameters
                    .required
                    .iter()
                    .any(|param| param.ast_type.contains_polymorph().is_some());

            let parameters = resolve_parameters(&type_ctx, &function.parameters)?;
            let return_type = type_ctx.resolve(&function.return_type)?;

            let constraints = if is_generic {
                let mut set = HashSet::default();

                for param in parameters.required.iter() {
                    collect_constraints(&mut set, &param.resolved_type);
                }

                collect_constraints(&mut set, &return_type);
                set
            } else {
                HashSet::default()
            };

            let function_ref = resolved_ast.functions.insert(resolved::Function {
                name: name.clone(),
                parameters,
                return_type,
                stmts: vec![],
                is_foreign: function.is_foreign,
                variables: VariableStorage::new(),
                source: function.source,
                abide_abi: function.abide_abi,
                tag: function.tag.or_else(|| {
                    if options.coerce_main_signature && &*function.name.basename == "main" {
                        Some(Tag::Main)
                    } else {
                        None
                    }
                }),
                is_generic,
                constraints,
            });

            if function.privacy.is_public() {
                let public_of_module = ctx.public_functions.entry(module_file_id).or_default();

                // TODO: Add proper error message
                let function_name = function
                    .name
                    .as_plain_str()
                    .expect("cannot make public symbol with existing namespace");

                if public_of_module.get(function_name).is_none() {
                    public_of_module.insert(function_name.to_string(), vec![]);
                }

                let functions_of_name = public_of_module
                    .get_mut(function_name)
                    .expect("function list inserted");
                functions_of_name.push(function_ref);
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

fn collect_constraints(set: &mut HashSet<Constraint>, ty: &resolved::Type) {
    match &ty.kind {
        resolved::TypeKind::Unresolved => panic!(),
        resolved::TypeKind::Boolean
        | resolved::TypeKind::Integer(_, _)
        | resolved::TypeKind::CInteger(_, _)
        | resolved::TypeKind::IntegerLiteral(_)
        | resolved::TypeKind::FloatLiteral(_)
        | resolved::TypeKind::Floating(_) => (),
        resolved::TypeKind::Pointer(inner) => collect_constraints(set, inner.as_ref()),
        resolved::TypeKind::Void => (),
        resolved::TypeKind::AnonymousStruct() => todo!(),
        resolved::TypeKind::AnonymousUnion() => todo!(),
        resolved::TypeKind::AnonymousEnum() => todo!(),
        resolved::TypeKind::FixedArray(fixed_array) => collect_constraints(set, &fixed_array.inner),
        resolved::TypeKind::FunctionPointer(_) => todo!(),
        resolved::TypeKind::Enum(_, _) => (),
        resolved::TypeKind::Structure(_, _) => (),
        resolved::TypeKind::TypeAlias(_, _) => (),
        resolved::TypeKind::Polymorph(_, constraints) => set.extend(constraints.iter().cloned()),
    }
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
