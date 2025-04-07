use super::error::LowerError;
use crate::ModBuilder;

pub fn lower_global(
    mod_builder: &ModBuilder,
    global_ref: asg::GlobalRef,
) -> Result<(), LowerError> {
    let global = mod_builder
        .asg
        .globals
        .get(global_ref)
        .expect("valid global reference");

    let mangled_name = if global.ownership.should_mangle() {
        global
            .name
            .display(&mod_builder.asg.workspace.fs)
            .to_string()
    } else {
        global.name.plain().to_string()
    };

    mod_builder.globals.insert(
        global_ref,
        ir::Global {
            mangled_name,
            ir_type: mod_builder.lower_type(&global.ty)?,
            is_thread_local: global.is_thread_local,
            ownership: global.ownership,
        },
    );

    Ok(())
}
