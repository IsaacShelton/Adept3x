use super::{ctx::ResolveCtx, error::ResolveError};
use asg::HelperExprDecl;
use ast_workspace::AstWorkspace;

pub fn resolve_helper_expressions(
    ctx: &mut ResolveCtx,
    ast_workspace: &AstWorkspace,
) -> Result<(), ResolveError> {
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_folder_id = ast_workspace.get_owning_module_or_self(*physical_file_id);

        for helper_expr in file.helper_exprs.iter() {
            let helper_exprs = ctx
                .helper_exprs_in_modules
                .entry(module_folder_id)
                .or_default();

            helper_exprs.insert(
                helper_expr.name.clone(),
                HelperExprDecl {
                    value: helper_expr.value.clone(),
                    privacy: helper_expr.privacy,
                },
            );
        }
    }

    Ok(())
}
