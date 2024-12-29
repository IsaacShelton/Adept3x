mod builder;
mod cast;
mod datatype;
mod error;
mod expr;
mod function;
mod global;
mod stmts;
mod structure;

use self::error::LowerError;
use crate::{
    asg::Asg,
    cli::BuildOptions,
    ir::{self},
    resolve::PolyRecipe,
};
use function::{lower_function_body, lower_function_head};
use global::lower_global;
use structure::lower_structure;

pub fn lower<'a>(options: &BuildOptions, rast: &Asg) -> Result<ir::Module, LowerError> {
    let mut ir_module = ir::Module::new(options.target.clone());

    for (struct_ref, structure) in rast.structs.iter() {
        lower_structure(&mut ir_module, struct_ref, structure, rast)?;
    }

    for (global_ref, global) in rast.globals.iter() {
        lower_global(&mut ir_module, global_ref, global, rast)?;
    }

    for (func_ref, function) in rast.funcs.iter() {
        if function.is_generic {
            continue;
        }

        ir_module
            .funcs
            .translate(func_ref, PolyRecipe::default(), || {
                lower_function_head(&ir_module, func_ref, &PolyRecipe::default(), rast)
            })?;
    }

    // Lower monomorphized functions
    let mut bodies = Vec::new();
    for (func_ref, poly_recipe, ir_func_ref) in ir_module.funcs.monomorphized() {
        bodies.push((
            *ir_func_ref,
            lower_function_body(&ir_module, *func_ref, &poly_recipe, rast)?,
        ));
    }

    for (ir_func_ref, basicblocks) in bodies {
        ir_module.funcs.get_mut(ir_func_ref).basicblocks = basicblocks;
    }

    if options.emit_ir {
        use std::{fs::File, io::Write};
        let mut f = File::create("out.ir").expect("failed to emit ir to file");
        writeln!(&mut f, "{:#?}", ir_module).expect("failed to write ir to file");
    }

    Ok(ir_module)
}
