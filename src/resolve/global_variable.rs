use super::{ctx::ResolveCtx, error::ResolveError, type_ctx::ResolveTypeCtx};
use crate::{
    ast::AstWorkspace,
    name::{Name, ResolvedName},
    resolved::{self, GlobalVarDecl},
};

pub fn resolve_global_variables(
    ctx: &mut ResolveCtx,
    resolved_ast: &mut resolved::Ast,
    ast_workspace: &AstWorkspace,
) -> Result<(), ResolveError> {
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

            ctx.globals_in_modules
                .entry(module_file_id)
                .or_default()
                .insert(
                    global.name.clone(),
                    GlobalVarDecl {
                        global_ref,
                        privacy: global.privacy,
                    },
                );
        }
    }

    Ok(())
}
