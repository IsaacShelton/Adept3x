use super::{
    ctx::ResolveCtx,
    error::ResolveError,
    expr::{resolve_expr, ResolveExprCtx},
    initialized::Initialized,
    variable_haystack::VariableHaystack,
};
use crate::{
    ast::AstWorkspace,
    resolved::{self, CurrentConstraints, HelperExprDecl},
};

pub fn resolve_helper_expressions(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    ast_workspace: &AstWorkspace,
) -> Result<(), ResolveError> {
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_file_id = ast_workspace
            .get_owning_module(*physical_file_id)
            .unwrap_or(*physical_file_id);

        let settings = &ast_workspace.settings[file.settings.unwrap_or_default().0];

        // NOTE: This module should already have a function haystack
        let function_haystack = ctx
            .function_haystacks
            .get(&module_file_id)
            .expect("function haystack to exist for file");

        for helper_expr in file.helper_exprs.iter() {
            let value = {
                let variable_haystack = VariableHaystack::new();
                let mut ctx = ResolveExprCtx {
                    resolved_ast,
                    function_haystack,
                    variable_haystack,
                    resolved_function_ref: None,
                    settings,
                    public_functions: &ctx.public_functions,
                    types_in_modules: &ctx.types_in_modules,
                    globals_in_modules: &ctx.globals_in_modules,
                    helper_exprs_in_modules: &ctx.helper_exprs_in_modules,
                    module_fs_node_id: module_file_id,
                    physical_fs_node_id: *physical_file_id,
                    current_constraints: CurrentConstraints {
                        constraints: Default::default(),
                        implementations: ctx.implementations,
                    },
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

    Ok(())
}
