use super::{
    ctx::ResolveCtx, error::ResolveError, expr::ResolveExprCtx, job::FuncJob, stmt::resolve_stmts,
    type_ctx::ResolveTypeCtx, variable_haystack::VariableHaystack,
};
use crate::{
    asg::{Asg, FuncRef},
    ast::{self, AstWorkspace},
    workspace::fs::FsNodeId,
};

pub fn resolve_function_bodies(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    ast_workspace: &AstWorkspace,
) -> Result<(), ResolveError> {
    while let Some(job) = ctx.jobs.pop_front() {
        match job {
            FuncJob::Regular(physical_file_id, ast_function_index, func_ref) => {
                let module_file_id = ast_workspace.get_owning_module_or_self(physical_file_id);

                let ast_file = ast_workspace
                    .files
                    .get(&physical_file_id)
                    .expect("file referenced by job to exist");

                let ast_function = ast_file
                    .funcs
                    .get(ast_function_index)
                    .expect("function referenced by job to exist");

                resolve_function_body(
                    ctx,
                    asg,
                    ast_workspace,
                    module_file_id,
                    physical_file_id,
                    ast_function,
                    func_ref,
                )?;
            }
            FuncJob::Impling(
                physical_file_id,
                ast_impl_index,
                ast_impl_function_index,
                func_ref,
            ) => {
                let module_file_id = ast_workspace.get_owning_module_or_self(physical_file_id);

                let ast_file = ast_workspace
                    .files
                    .get(&physical_file_id)
                    .expect("file referenced by job to exist");

                let ast_function = ast_file
                    .impls
                    .get(ast_impl_index)
                    .expect("referenced impl to exist")
                    .body
                    .get(ast_impl_function_index)
                    .expect("referenced impl function to exist");

                resolve_function_body(
                    ctx,
                    asg,
                    ast_workspace,
                    module_file_id,
                    physical_file_id,
                    ast_function,
                    func_ref,
                )?;
            }
        }
    }

    Ok(())
}

fn resolve_function_body(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    ast_workspace: &AstWorkspace,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    ast_function: &ast::Func,
    func_ref: FuncRef,
) -> Result<(), ResolveError> {
    let function_haystack = ctx
        .function_haystacks
        .get(&module_file_id)
        .expect("function haystack to exist for file");

    let variable_haystack = resolve_parameter_variables(
        ctx,
        asg,
        module_file_id,
        physical_file_id,
        ast_function,
        func_ref,
    )?;

    let file = ast_workspace
        .files
        .get(&physical_file_id)
        .expect("referenced file exists");

    let settings = &ast_workspace.settings[file.settings.unwrap_or_default().0];

    let f = asg
        .funcs
        .get(func_ref)
        .expect("referenced resolved function to exist");

    let constraints = f.constraints.clone();

    let resolved_stmts = resolve_stmts(
        &mut ResolveExprCtx {
            asg,
            function_haystack,
            variable_haystack,
            func_ref: Some(func_ref),
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

    asg.funcs
        .get_mut(func_ref)
        .expect("resolved function head to exist")
        .stmts = resolved_stmts;

    Ok(())
}

fn resolve_parameter_variables(
    ctx: &ResolveCtx,
    asg: &mut Asg,
    module_file_id: FsNodeId,
    physical_file_id: FsNodeId,
    ast_function: &ast::Func,
    func_ref: FuncRef,
) -> Result<VariableHaystack, ResolveError> {
    let mut variable_haystack = VariableHaystack::new();

    for parameter in ast_function.head.params.required.iter() {
        let function = asg.funcs.get(func_ref).unwrap();

        let type_ctx = ResolveTypeCtx::new(
            &asg,
            module_file_id,
            physical_file_id,
            &ctx.types_in_modules,
            &function.constraints,
        );

        let mut ty = type_ctx.resolve(&parameter.ast_type)?;
        ty.strip_constraints();

        let function = asg.funcs.get_mut(func_ref).unwrap();

        let key = function.variables.add_parameter(ty.clone());
        variable_haystack.put(parameter.name.clone(), ty, key);
    }

    Ok(variable_haystack)
}
