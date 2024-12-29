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

    for (structure_ref, structure) in rast.structures.iter() {
        lower_structure(&mut ir_module, structure_ref, structure, rast)?;
    }

    for (global_ref, global) in rast.globals.iter() {
        lower_global(&mut ir_module, global_ref, global, rast)?;
    }

    for (function_ref, function) in rast.functions.iter() {
        if function.is_generic {
            continue;
        }

        ir_module
            .functions
            .translate(function_ref, PolyRecipe::default(), || {
                lower_function_head(&ir_module, function_ref, &PolyRecipe::default(), rast)
            })?;
    }

    // Lower monomorphized functions
    let mut bodies = Vec::new();
    for (function_ref, poly_recipe, ir_function_ref) in ir_module.functions.monomorphized() {
        bodies.push((
            *ir_function_ref,
            lower_function_body(&ir_module, *function_ref, &poly_recipe, rast)?,
        ));
    }

    for (ir_function_ref, basicblocks) in bodies {
        ir_module.functions.get_mut(ir_function_ref).basicblocks = basicblocks;
    }

    if options.emit_ir {
        use std::{fs::File, io::Write};
        let mut f = File::create("out.ir").expect("failed to emit ir to file");
        writeln!(&mut f, "{:#?}", ir_module).expect("failed to write ir to file");
    }

    Ok(ir_module)
}
