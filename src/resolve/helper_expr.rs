use super::{
    ctx::ResolveCtx,
    error::ResolveError,
    expr::{resolve_expr, ResolveExprCtx, ResolveExprMode},
    initialized::Initialized,
    variable_haystack::VariableHaystack,
};
use crate::{
    asg::{Asg, HelperExprDecl},
    ast::AstWorkspace,
};

pub fn resolve_helper_expressions(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    ast_workspace: &AstWorkspace,
) -> Result<(), ResolveError> {
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_file_id = ast_workspace.get_owning_module_or_self(*physical_file_id);
        let settings = &ast_workspace.settings[file.settings.unwrap_or_default().0];

        // NOTE: This module should already have a function haystack
        let func_haystack = ctx
            .func_haystacks
            .get(&module_file_id)
            .expect("function haystack to exist for file");

        for helper_expr in file.helper_exprs.iter() {
            let value = {
                let variable_haystack = VariableHaystack::new();
                let mut ctx = ResolveExprCtx {
                    asg,
                    func_haystack,
                    variable_haystack,
                    func_ref: None,
                    settings,
                    public_funcs: &ctx.public_funcs,
                    types_in_modules: &ctx.types_in_modules,
                    globals_in_modules: &ctx.globals_in_modules,
                    helper_exprs_in_modules: &ctx.helper_exprs_in_modules,
                    impls_in_modules: &ctx.impls_in_modules,
                    module_fs_node_id: module_file_id,
                    physical_fs_node_id: *physical_file_id,
                };

                resolve_expr(
                    &mut ctx,
                    &helper_expr.value,
                    None,
                    Initialized::Require,
                    ResolveExprMode::RequireValue,
                )?
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

    Ok(())
}
