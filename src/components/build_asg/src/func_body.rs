use super::{
    ctx::ResolveCtx,
    error::ResolveError,
    expr::{ResolveExprCtx, ResolveExprMode},
    job::FuncJob,
    stmt::resolve_stmts,
    type_ctx::{ResolveTypeCtx, ResolveTypeOptions},
    variable_haystack::VariableHaystack,
};
use asg::{Asg, FuncRef};
use ast_workspace::AstWorkspace;
use fs_tree::FsNodeId;

pub fn resolve_func_bodies(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    ast_workspace: &AstWorkspace,
) -> Result<(), ResolveError> {
    while let Some(job) = ctx.jobs.pop_front() {
        match job {
            FuncJob::Regular(physical_file_id, ast_func_index, func_ref) => {
                let module_folder_id = ast_workspace.get_owning_module_or_self(physical_file_id);

                let ast_function = &ast_workspace.symbols.all_funcs[ast_func_index];

                resolve_func_body(
                    ctx,
                    asg,
                    ast_workspace,
                    module_folder_id,
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
                let module_folder_id = ast_workspace.get_owning_module_or_self(physical_file_id);

                let ast_function = ast_workspace.symbols.all_impls[ast_impl_index]
                    .body
                    .get(ast_impl_function_index)
                    .expect("referenced impl function to exist");

                resolve_func_body(
                    ctx,
                    asg,
                    ast_workspace,
                    module_folder_id,
                    physical_file_id,
                    ast_function,
                    func_ref,
                )?;
            }
        }
    }

    Ok(())
}

fn resolve_func_body(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    ast_workspace: &AstWorkspace,
    module_folder_id: FsNodeId,
    physical_file_id: FsNodeId,
    ast_function: &ast::Func,
    func_ref: FuncRef,
) -> Result<(), ResolveError> {
    let func_haystack = ctx
        .func_haystacks
        .get(&module_folder_id)
        .expect("function haystack to exist for file");

    let variable_haystack = resolve_param_vars(
        ctx,
        asg,
        module_folder_id,
        physical_file_id,
        ast_function,
        func_ref,
    )?;

    let file = ast_workspace
        .files
        .get(physical_file_id)
        .expect("referenced file exists");

    let settings = &ast_workspace.settings[file.settings.unwrap_or(ast_workspace.default_settings)];

    let resolved_stmts = resolve_stmts(
        &mut ResolveExprCtx {
            asg,
            func_haystack,
            variable_haystack,
            func_ref: Some(func_ref),
            settings,
            public_funcs: &ctx.public_funcs,
            types_in_modules: &ctx.types_in_modules,
            globals_in_modules: &ctx.globals_in_modules,
            helper_exprs_in_modules: &mut ctx.helper_exprs_in_modules,
            impls_in_modules: &mut ctx.impls_in_modules,
            module_fs_node_id: module_folder_id,
            physical_fs_node_id: physical_file_id,
        },
        &ast_function.stmts,
        ResolveExprMode::NeglectValue,
    )?;

    asg.funcs[func_ref].stmts = resolved_stmts;
    Ok(())
}

fn resolve_param_vars(
    ctx: &ResolveCtx,
    asg: &mut Asg,
    module_folder_id: FsNodeId,
    physical_file_id: FsNodeId,
    ast_func: &ast::Func,
    func_ref: FuncRef,
) -> Result<VariableHaystack, ResolveError> {
    let mut variable_haystack = VariableHaystack::new();

    for param in ast_func.head.params.required.iter() {
        let type_ctx = ResolveTypeCtx::new(
            &asg,
            module_folder_id,
            physical_file_id,
            &ctx.types_in_modules,
        );

        let ty = type_ctx.resolve(&param.ast_type, ResolveTypeOptions::Unalias)?;
        let key = asg.funcs[func_ref].vars.add_param(ty.clone());

        if let Some(name) = &param.name {
            variable_haystack.put(name.clone(), ty, key);
        }
    }

    Ok(variable_haystack)
}
