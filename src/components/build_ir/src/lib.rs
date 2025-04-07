mod cast;
mod datatype;
mod error;
mod expr;
mod func_builder;
mod funcs;
mod function;
mod global;
mod globals;
mod stmts;
mod structs;
mod structure;

use asg::{Asg, PolyRecipe};
use compiler::BuildOptions;
use data_units::ByteUnits;
use datatype::lower_type;
use error::LowerError;
use func_builder::unpoly;
use funcs::Funcs;
use function::{lower_func_body, lower_func_head};
use global::lower_global;
use globals::Globals;
use structs::Structs;
use structure::lower_struct;
use target::Target;
use target_layout::TargetLayout;

pub struct ModBuilder<'a> {
    asg: &'a Asg<'a>,
    target: Target,
    structs: Structs,
    globals: Globals,
    funcs: Funcs,
}

impl<'a> ModBuilder<'a> {
    pub fn build(self) -> ir::Module {
        ir::Module {
            interpreter_entry_point: self.funcs.interpreter_entry_point(),
            target: self.target,
            structs: self.structs.build(),
            globals: self.globals.build(),
            funcs: self.funcs.build(),
        }
    }

    pub fn lower_type(&self, ty: &asg::Type) -> Result<ir::Type, LowerError> {
        lower_type(self, &unpoly(&PolyRecipe::default(), ty)?)
    }
}

pub fn lower<'a>(options: &BuildOptions, asg: &Asg) -> Result<ir::Module, LowerError> {
    let mut mod_builder = ModBuilder {
        asg,
        target: options.target.clone(),
        structs: Structs::default(),
        globals: Globals::default(),
        funcs: Funcs::default(),
    };

    assert_eq!(
        mod_builder.target.short_layout().width,
        ByteUnits::of(2),
        "This target is not supported. Adept currently assumes that shorts are 16-bit integers (for integer promotion rules). Which does not hold for this target."
    );

    assert_eq!(
        mod_builder.target.int_layout().width,
        ByteUnits::of(4),
        "This target is not supported. Adept currently assumes that ints are 32-bit integers (for integer promotion rules). Which does not hold for this target."
    );

    for struct_ref in asg.structs.keys() {
        lower_struct(&mut mod_builder, struct_ref)?;
    }

    for global_ref in asg.globals.keys() {
        lower_global(&mut mod_builder, global_ref)?;
    }

    for (func_ref, function) in asg.funcs.iter() {
        if function.is_generic {
            continue;
        }

        mod_builder
            .funcs
            .translate(asg, func_ref, PolyRecipe::default(), || {
                lower_func_head(&mod_builder, func_ref, &PolyRecipe::default())
            })?;
    }

    // Lower monomorphized functions
    let mut bodies = Vec::new();
    for (asg_func_ref, poly_recipe, ir_func_ref) in mod_builder.funcs.monomorphized() {
        bodies.push((
            *ir_func_ref,
            lower_func_body(&mod_builder, *asg_func_ref, &poly_recipe, asg)?,
        ));
    }

    for (ir_func_ref, basicblocks) in bodies {
        mod_builder.funcs.get_mut(ir_func_ref).basicblocks = basicblocks;
    }

    let ir_module = mod_builder.build();

    if options.emit_ir {
        use std::{fs::File, io::Write};
        let mut f = File::create("out.ir").expect("failed to emit ir to file");
        writeln!(&mut f, "{:#?}", ir_module).expect("failed to write ir to file");
    }

    Ok(ir_module)
}
