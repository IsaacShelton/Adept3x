mod conform;
mod core_structure_info;
mod ctx;
mod destination;
mod error;
mod expr;
mod function_search_ctx;
mod initialized;
mod job;
mod stmt;
mod type_ctx;
mod unify_types;
mod variable_search_ctx;

use self::{
    error::ResolveError, expr::ResolveExprCtx, stmt::resolve_stmts,
    variable_search_ctx::VariableSearchCtx,
};
use crate::{
    ast::{self, AstWorkspace},
    cli::BuildOptions,
    index_map_ext::IndexMapExt,
    name::{Name, ResolvedName},
    resolved::{
        self, GlobalVarDecl, HelperExprDecl, HumanName, TypeDecl, TypeKind, VariableStorage,
    },
    tag::Tag,
};
use ctx::ResolveCtx;
use expr::resolve_expr;
use function_search_ctx::FunctionSearchCtx;
use indexmap::IndexMap;
use initialized::Initialized;
use job::{FuncJob, TypeJob};
use std::{borrow::Cow, collections::HashMap};
use type_ctx::ResolveTypeCtx;

pub fn prepare_type_jobs(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    ast_workspace: &AstWorkspace,
) -> Result<Vec<TypeJob>, ResolveError> {
    let mut type_jobs = Vec::with_capacity(ast_workspace.files.len());

    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_fs_node_id = ast_workspace
            .get_owning_module(*physical_file_id)
            .unwrap_or(*physical_file_id);

        let mut job = TypeJob {
            physical_file_id: *physical_file_id,
            type_aliases: Vec::with_capacity(file.type_aliases.len()),
            structures: Vec::with_capacity(file.structures.len()),
            enums: Vec::with_capacity(file.enums.len()),
        };

        for structure in file.structures.iter() {
            let privacy = structure.privacy;
            let source = structure.source;
            let resolved_name = ResolvedName::new(module_fs_node_id, &structure.name);

            let structure_ref = resolved_ast.structures.insert(resolved::Structure {
                name: resolved_name.clone(),
                fields: IndexMap::new(),
                is_packed: structure.is_packed,
                source: structure.source,
            });

            let struct_type_kind =
                TypeKind::Structure(HumanName(structure.name.to_string()), structure_ref);

            let Some(name) = structure.name.as_plain_str() else {
                eprintln!(
                    "warning: internal namespaced structures ignored by new type resolution system"
                );
                continue;
            };

            let types_in_module = ctx
                .types_in_modules
                .entry(module_fs_node_id)
                .or_insert_with(HashMap::new);

            types_in_module.insert(
                name.to_string(),
                TypeDecl {
                    kind: struct_type_kind,
                    source,
                    privacy,
                },
            );

            job.structures.push(structure_ref);
        }

        for definition in file.enums.iter() {
            let enum_ref = resolved_ast.enums.insert(resolved::Enum {
                name: ResolvedName::new(module_fs_node_id, &Name::plain(&definition.name)),
                resolved_type: TypeKind::Unresolved.at(definition.source),
                source: definition.source,
                members: definition.members.clone(),
            });

            let kind = TypeKind::Enum(HumanName(definition.name.to_string()), enum_ref);
            let source = definition.source;
            let privacy = definition.privacy;

            let types_in_module = ctx
                .types_in_modules
                .entry(module_fs_node_id)
                .or_insert_with(HashMap::new);

            types_in_module.insert(
                definition.name.to_string(),
                TypeDecl {
                    kind,
                    source,
                    privacy,
                },
            );

            job.enums.push(enum_ref);
        }

        for definition in file.type_aliases.iter() {
            let type_alias_ref = resolved_ast
                .type_aliases
                .insert(resolved::TypeKind::Unresolved.at(definition.value.source));

            let source = definition.source;
            let privacy = definition.privacy;
            let kind = TypeKind::TypeAlias(HumanName(definition.name.to_string()), type_alias_ref);

            let types_in_module = ctx
                .types_in_modules
                .entry(module_fs_node_id)
                .or_insert_with(HashMap::new);

            types_in_module.insert(
                definition.name.to_string(),
                TypeDecl {
                    kind,
                    source,
                    privacy,
                },
            );

            job.type_aliases.push(type_alias_ref);
        }

        type_jobs.push(job);
    }

    Ok(type_jobs)
}

pub fn process_type_jobs(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    ast_workspace: &AstWorkspace,
    type_jobs: &[TypeJob],
) -> Result<(), ResolveError> {
    for job in type_jobs.iter() {
        let file = ast_workspace
            .files
            .get(&job.physical_file_id)
            .expect("valid ast file");

        let module_file_id = ast_workspace
            .get_owning_module(job.physical_file_id)
            .unwrap_or(job.physical_file_id);

        for (structure_ref, structure) in job.structures.iter().zip(file.structures.iter()) {
            for (field_name, field) in structure.fields.iter() {
                let type_ctx = ResolveTypeCtx::new(
                    &resolved_ast,
                    module_file_id,
                    job.physical_file_id,
                    &ctx.types_in_modules,
                );

                let resolved_type = type_ctx.resolve_or_undeclared(&field.ast_type)?;

                let resolved_struct = resolved_ast
                    .structures
                    .get_mut(*structure_ref)
                    .expect("valid struct");

                resolved_struct.fields.insert(
                    field_name.clone(),
                    resolved::Field {
                        resolved_type,
                        privacy: field.privacy,
                        source: field.source,
                    },
                );
            }
        }

        for (enum_ref, definition) in job.enums.iter().zip(file.enums.iter()) {
            let type_ctx = ResolveTypeCtx::new(
                &resolved_ast,
                module_file_id,
                job.physical_file_id,
                &ctx.types_in_modules,
            );

            let ast_type = definition
                .backing_type
                .as_ref()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| Cow::Owned(ast::TypeKind::u32().at(definition.source)));

            let resolved_type = type_ctx.resolve_or_undeclared(&ast_type)?;

            let definition = resolved_ast.enums.get_mut(*enum_ref).unwrap();
            definition.resolved_type = resolved_type;
        }

        for (type_alias_ref, definition) in job.type_aliases.iter().zip(file.type_aliases.iter()) {
            let type_ctx = ResolveTypeCtx::new(
                &resolved_ast,
                module_file_id,
                job.physical_file_id,
                &ctx.types_in_modules,
            );

            let resolved_type = type_ctx.resolve_or_undeclared(&definition.value)?;

            let binding = resolved_ast.type_aliases.get_mut(*type_alias_ref).unwrap();
            *binding = resolved_type;
        }
    }

    Ok(())
}

pub fn resolve<'a>(
    ast_workspace: &'a AstWorkspace,
    options: &BuildOptions,
) -> Result<resolved::Ast<'a>, ResolveError> {
    let mut ctx = ResolveCtx::new();
    let source_files = ast_workspace.source_files;
    let mut resolved_ast = resolved::Ast::new(source_files, &ast_workspace);

    prepare_type_jobs(&mut ctx, &mut resolved_ast, ast_workspace).and_then(|type_jobs| {
        process_type_jobs(
            &mut ctx,
            &mut resolved_ast,
            ast_workspace,
            type_jobs.as_slice(),
        )
    })?;

    // Resolve global variables
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_file_id = ast_workspace
            .get_owning_module(*physical_file_id)
            .unwrap_or(*physical_file_id);

        for global in file.global_variables.iter() {
            let type_ctx = ResolveTypeCtx::new(
                &resolved_ast,
                module_file_id,
                *physical_file_id,
                &ctx.types_in_modules,
            );
            let resolved_type = type_ctx.resolve(&global.ast_type)?;

            let resolved_name = ResolvedName::new(module_file_id, &Name::plain(&global.name));

            let global_ref = resolved_ast.globals.insert(resolved::GlobalVar {
                name: resolved_name,
                resolved_type: resolved_type.clone(),
                source: global.source,
                is_foreign: global.is_foreign,
                is_thread_local: global.is_thread_local,
            });

            let globals = ctx.globals_in_modules.entry(module_file_id).or_default();

            globals.insert(
                global.name.clone(),
                GlobalVarDecl {
                    global_ref,
                    privacy: global.privacy,
                },
            );
        }
    }

    // Create initial function jobs
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
            let parameters = resolve_parameters(&type_ctx, &function.parameters)?;
            let return_type = type_ctx.resolve(&function.return_type)?;

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
            });

            ctx.jobs.push_back(FuncJob::Regular(
                *physical_file_id,
                function_i,
                function_ref,
            ));

            if function.privacy.is_public() {
                let public_of_module = ctx
                    .public_functions
                    .entry(module_file_id)
                    .or_insert_with(HashMap::new);

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
                ctx.function_search_ctxs
                    .get_or_insert_with(module_file_id, || {
                        FunctionSearchCtx::new(
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
        }
    }

    // Resolve helper expressions
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_file_id = ast_workspace
            .get_owning_module(*physical_file_id)
            .unwrap_or(*physical_file_id);

        let settings = &ast_workspace.settings[file.settings.unwrap_or_default().0];

        // NOTE: This module should already have a function search context
        let function_search_ctx = ctx
            .function_search_ctxs
            .get(&module_file_id)
            .expect("function search context to exist for file");

        for helper_expr in file.helper_exprs.iter() {
            let value = {
                let variable_search_ctx = VariableSearchCtx::new();
                let mut ctx = ResolveExprCtx {
                    resolved_ast: &mut resolved_ast,
                    function_search_ctx,
                    variable_search_ctx,
                    resolved_function_ref: None,
                    settings,
                    public_functions: &ctx.public_functions,
                    types_in_modules: &ctx.types_in_modules,
                    globals_in_modules: &ctx.globals_in_modules,
                    helper_exprs_in_modules: &ctx.helper_exprs_in_modules,
                    module_fs_node_id: module_file_id,
                    physical_fs_node_id: *physical_file_id,
                };

                resolve_expr(&mut ctx, &helper_expr.value, None, Initialized::Require)?
            };

            let helper_exprs = ctx
                .helper_exprs_in_modules
                .entry(module_file_id)
                .or_default();

            helper_exprs.insert(
                helper_expr.name.clone(),
                HelperExprDecl {
                    value,
                    privacy: helper_expr.privacy,
                },
            );
        }
    }

    // Resolve function bodies
    while let Some(job) = ctx.jobs.pop_front() {
        match job {
            FuncJob::Regular(real_file_id, function_index, resolved_function_ref) => {
                let module_file_id = ast_workspace
                    .get_owning_module(real_file_id)
                    .unwrap_or(real_file_id);

                // NOTE: This module should already have a function search context
                let function_search_ctx = ctx
                    .function_search_ctxs
                    .get(&module_file_id)
                    .expect("function search context to exist for file");

                let ast_file = ast_workspace
                    .files
                    .get(&real_file_id)
                    .expect("file referenced by job to exist");

                let ast_function = ast_file
                    .functions
                    .get(function_index)
                    .expect("function referenced by job to exist");

                let mut variable_search_ctx = VariableSearchCtx::new();

                {
                    for parameter in ast_function.parameters.required.iter() {
                        let type_ctx = ResolveTypeCtx::new(
                            &resolved_ast,
                            module_file_id,
                            real_file_id,
                            &ctx.types_in_modules,
                        );

                        let resolved_type = type_ctx.resolve(&parameter.ast_type)?;

                        let function = resolved_ast
                            .functions
                            .get_mut(resolved_function_ref)
                            .unwrap();

                        let variable_key = function.variables.add_parameter(resolved_type.clone());

                        variable_search_ctx.put(
                            parameter.name.clone(),
                            resolved_type,
                            variable_key,
                        );
                    }
                }

                let file = ast_workspace
                    .files
                    .get(&real_file_id)
                    .expect("referenced file exists");

                let settings = &ast_workspace.settings[file.settings.unwrap_or_default().0];

                let resolved_stmts = {
                    let mut ctx = ResolveExprCtx {
                        resolved_ast: &mut resolved_ast,
                        function_search_ctx,
                        variable_search_ctx,
                        resolved_function_ref: Some(resolved_function_ref),
                        settings,
                        public_functions: &ctx.public_functions,
                        types_in_modules: &ctx.types_in_modules,
                        globals_in_modules: &ctx.globals_in_modules,
                        helper_exprs_in_modules: &mut ctx.helper_exprs_in_modules,
                        module_fs_node_id: module_file_id,
                        physical_fs_node_id: real_file_id,
                    };

                    resolve_stmts(&mut ctx, &ast_function.stmts)?
                };

                let resolved_function = resolved_ast
                    .functions
                    .get_mut(resolved_function_ref)
                    .expect("resolved function head to exist");

                resolved_function.stmts = resolved_stmts;
            }
        }
    }

    Ok(resolved_ast)
}

fn resolve_parameters(
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
