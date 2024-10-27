use super::{error::LowerError, lower_type};
use crate::{
    ir::{self, Global},
    resolved,
};

pub fn lower_global(
    ir_module: &mut ir::Module,
    global_ref: resolved::GlobalVarRef,
    global: &resolved::GlobalVar,
    resolved_ast: &resolved::Ast,
) -> Result<(), LowerError> {
    let mangled_name = if global.is_foreign {
        global.name.plain().to_string()
    } else {
        global.name.display(&resolved_ast.workspace.fs).to_string()
    };

    ir_module.globals.insert(
        global_ref,
        Global {
            mangled_name,
            ir_type: lower_type(&ir_module.target, &global.resolved_type, resolved_ast)?,
            is_foreign: global.is_foreign,
            is_thread_local: global.is_thread_local,
        },
    );

    Ok(())
}
