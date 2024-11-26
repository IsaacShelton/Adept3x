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
    cli::BuildOptions,
    ir::{self},
    resolved::{self, PolyRecipe},
    target::Target,
};
use function::{lower_function_body, lower_function_head};
use global::lower_global;
use structure::lower_structure;

pub fn lower<'a>(
    options: &BuildOptions,
    ast: &resolved::Ast,
    target: &'a Target,
) -> Result<ir::Module<'a>, LowerError> {
    let mut ir_module = ir::Module::new(target);

    for (structure_ref, structure) in ast.structures.iter() {
        lower_structure(&mut ir_module, structure_ref, structure, ast)?;
    }

    for (global_ref, global) in ast.globals.iter() {
        lower_global(&mut ir_module, global_ref, global, ast)?;
    }

    for (function_ref, function) in ast.functions.iter() {
        if function.is_generic {
            continue;
        }

        lower_function_head(&mut ir_module, function_ref, &PolyRecipe::default(), ast)?;
    }

    for (function_ref, function) in ast.functions.iter() {
        if function.is_generic {
            continue;
        }

        lower_function_body(&mut ir_module, function_ref, &PolyRecipe::default(), ast)?;
    }

    if options.emit_ir {
        use std::{fs::File, io::Write};
        let mut f = File::create("out.ir").expect("failed to emit ir to file");
        writeln!(&mut f, "{:#?}", ir_module).expect("failed to write ir to file");
    }

    Ok(ir_module)
}
