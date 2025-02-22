use super::{
    ctx::ResolveCtx,
    error::ResolveError,
    type_ctx::{ResolveTypeCtx, ResolveTypeOptions},
};
use crate::{
    asg::{self, Asg, GlobalVarDecl},
    ast::AstWorkspace,
    name::{Name, ResolvedName},
};

pub fn resolve_global_variables(
    ctx: &mut ResolveCtx,
    asg: &mut Asg,
    ast_workspace: &AstWorkspace,
) -> Result<(), ResolveError> {
    for (physical_file_id, file) in ast_workspace.files.iter() {
        let module_folder_id = ast_workspace.get_owning_module_or_self(*physical_file_id);

        for global in file.global_variables.iter() {
            let type_ctx = ResolveTypeCtx::new(
                &asg,
                module_folder_id,
                *physical_file_id,
                &ctx.types_in_modules,
            );

            let ty = type_ctx.resolve(&global.ast_type, ResolveTypeOptions::Unalias)?;
            let resolved_name = ResolvedName::new(module_folder_id, &Name::plain(&global.name));

            let global_ref = asg.globals.insert(asg::GlobalVar {
                name: resolved_name,
                ty: ty.clone(),
                source: global.source,
                is_foreign: global.is_foreign,
                is_thread_local: global.is_thread_local,
            });

            ctx.globals_in_modules
                .entry(module_folder_id)
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
