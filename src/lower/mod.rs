use crate::{ast::Ast, ir};

pub fn lower(ast: &Ast) -> ir::Module {
    let mut ir_module = ir::Module::new();

    for function in ast.functions.iter() {
        ir_module.functions.insert(ir::Function {
            mangled_name: function.name.clone(),
            basicblocks: vec![],
            parameters: vec![],
            return_type: ir::Type::Void,
            is_cstyle_variadic: false,
            is_foreign: true,
            is_exposed: true,
        });
    }

    ir_module
}

