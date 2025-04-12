use super::{ctx::ResolveCtx, error::ResolveError};
use asg::HelperExprDecl;
use ast_workspace::AstWorkspace;

pub fn resolve_expr_aliases(
    ctx: &mut ResolveCtx,
    ast_workspace: &AstWorkspace,
) -> Result<(), ResolveError> {
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_folder_id = ast_workspace.get_owning_module_or_self(physical_file_id);

        for helper_expr in ast_workspace.view(file).expr_aliases {
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
