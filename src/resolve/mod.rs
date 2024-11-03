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
mod type_definition;
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
    resolved::{self, GlobalVarDecl, HelperExprDecl, VariableStorage},
    tag::Tag,
};
use ctx::ResolveCtx;
use expr::resolve_expr;
use function_search_ctx::FunctionSearchCtx;
use initialized::Initialized;
use job::FuncJob;
use std::collections::HashMap;
use type_ctx::ResolveTypeCtx;
use type_definition::resolve_type_definitions;

pub fn resolve<'a>(
    ast_workspace: &'a AstWorkspace,
    options: &BuildOptions,
) -> Result<resolved::Ast<'a>, ResolveError> {
    let mut ctx = ResolveCtx::new();
    let source_files = ast_workspace.source_files;
    let mut resolved_ast = resolved::Ast::new(source_files, &ast_workspace);

    resolve_type_definitions(&mut ctx, &mut resolved_ast, ast_workspace)?;

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
                is_generic: false,
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
