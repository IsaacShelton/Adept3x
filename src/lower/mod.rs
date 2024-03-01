mod builder;

use crate::{
    ast::{Ast, Expression, Statement},
    error::CompilerError,
    ir::{self, BasicBlocks, Literal},
};
use builder::Builder;

pub fn lower(ast: &Ast) -> Result<ir::Module, CompilerError> {
    let mut ir_module = ir::Module::new();

    for function in ast.functions.iter() {
        let mut builder = Builder::new();

        for statement in function.statements.iter() {
            match statement {
                Statement::Return(expression) => {
                    let instruction = ir::Instruction::Return(
                        if let Some(expression) = expression {
                            Some(lower_expression(&mut builder, expression)?)
                        } else {
                            None
                        },
                    );

                    builder.push(instruction);
                }
            }
        }

        builder.terminate();
        let basicblocks = builder.build();

        ir_module.functions.insert(ir::Function {
            mangled_name: function.name.clone(),
            basicblocks,
            parameters: vec![],
            return_type: ir::Type::Void,
            is_cstyle_variadic: false,
            is_foreign: true,
            is_exposed: true,
        });
    }

    Ok(ir_module)
}

fn lower_expression(
    builder: &mut Builder,
    expression: &Expression,
) -> Result<ir::Value, CompilerError> {
    match expression {
        Expression::Integer(value) => if let Ok(value) = value.try_into() {
            Ok(ir::Value::Literal(Literal::Signed64(value)))
        } else {
            Err(CompilerError::during_lower("Integer literal does not fit within signed 64-bit integer"))
        },
    }
}
