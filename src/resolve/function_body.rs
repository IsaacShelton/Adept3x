use super::{
    ctx::ResolveCtx, error::ResolveError, expr::ResolveExprCtx, job::FuncJob, stmt::resolve_stmts,
    type_ctx::ResolveTypeCtx, variable_haystack::VariableHaystack,
};
use crate::{
    ast::{self, AstWorkspace},
    resolved::{self, FunctionRef},
    workspace::fs::FsNodeId,
};

pub fn resolve_function_bodies(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    ast_workspace: &AstWorkspace,
) -> Result<(), ResolveError> {
    while let Some(job) = ctx.jobs.pop_front() {
        match job {
            FuncJob::Regular(physical_file_id, ast_function_index, resolved_function_ref) => {
                let module_file_id = ast_workspace
                    .get_owning_module(physical_file_id)
                    .unwrap_or(physical_file_id);

                let function_haystack = ctx
                    .function_haystacks
                    .get(&module_file_id)
                    .expect("function haystack to exist for file");

                let ast_file = ast_workspace
                    .files
                    .get(&physical_file_id)
                    .expect("file referenced by job to exist");

                let ast_function = ast_file
                    .functions
                    .get(ast_function_index)
                    .expect("function referenced by job to exist");

                let variable_haystack = resolve_parameter_variables(
                    ctx,
                    resolved_ast,
                    module_file_id,
                    physical_file_id,
                    ast_function,
                    resolved_function_ref,
                )?;

                let file = ast_workspace
                    .files
                    .get(&physical_file_id)
                    .expect("referenced file exists");

                let settings = &ast_workspace.settings[file.settings.unwrap_or_default().0];

                let f = resolved_ast
                    .functions
                    .get(resolved_function_ref)
                    .expect("referenced resolved function to exist");
                let constraints = f.constraints.clone();

                let resolved_stmts = resolve_stmts(
                    &mut ResolveExprCtx {
                        resolved_ast,
                        function_haystack,
                        variable_haystack,
                        resolved_function_ref: Some(resolved_function_ref),
                        settings,
                        public_functions: &ctx.public_functions,
                        types_in_modules: &ctx.types_in_modules,
                        globals_in_modules: &ctx.globals_in_modules,
                        helper_exprs_in_modules: &mut ctx.helper_exprs_in_modules,
                        module_fs_node_id: module_file_id,
                        physical_fs_node_id: physical_file_id,
                        current_constraints: constraints,
                    },
                    &ast_function.stmts,
                )?;

                resolved_ast
                    .functions
                    .get_mut(resolved_function_ref)
                    .expect("resolved function head to exist")
                    .stmts = resolved_stmts;
            }
        }
    }

    Ok(())
}

fn resolve_parameter_variables(
    ctx: &ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    ast_function: &ast::Function,
    resolved_function_ref: FunctionRef,
) -> Result<VariableHaystack, ResolveError> {
    let mut variable_haystack = VariableHaystack::new();

    for parameter in ast_function.parameters.required.iter() {
        let function = resolved_ast.functions.get(resolved_function_ref).unwrap();

        let type_ctx = ResolveTypeCtx::new(
            &resolved_ast,
            module_file_id,
            physical_file_id,
            &ctx.types_in_modules,
            &function.constraints,
        );

        let mut resolved_type = type_ctx.resolve(&parameter.ast_type)?;
        resolved_type.strip_constraints();

        let function = resolved_ast
            .functions
            .get_mut(resolved_function_ref)
            .unwrap();

        let key = function.variables.add_parameter(resolved_type.clone());
        variable_haystack.put(parameter.name.clone(), resolved_type, key);
    }

    Ok(variable_haystack)
}
