mod builder;

use crate::{
    ast::{self, Ast, Expression, Statement},
    error::CompilerError,
    ir::{self, BasicBlocks, Literal},
};
use builder::Builder;

pub fn lower(ast: &Ast) -> Result<ir::Module, CompilerError> {
    let mut ir_module = ir::Module::new();

    for (file_identifier, file) in ast.files.iter() {
        for function in file.functions.iter() {
            lower_function(&mut ir_module, function);
        }
    }

    Ok(ir_module)
}

fn lower_function(
    ir_module: &mut ir::Module,
    function: &ast::Function,
) -> Result<(), CompilerError> {
    let basicblocks = if !function.is_foreign {
        let mut builder = Builder::new();

        for statement in function.statements.iter() {
            match statement {
                Statement::Return(expression) => {
                    let instruction =
                        ir::Instruction::Return(if let Some(expression) = expression {
                            Some(lower_expression(&mut builder, expression)?)
                        } else {
                            None
                        });

                    builder.push(instruction);
                }
            }
        }

        builder.terminate();
        builder.build()
    } else {
        BasicBlocks::default()
    };

    let mut parameters = vec![];
    for parameter in function.parameters.iter() {
        parameters.push(lower_type(&parameter.ast_type)?);
    }

    ir_module.functions.insert(ir::Function {
        mangled_name: function.name.clone(),
        basicblocks,
        parameters, 
        return_type: lower_type(&function.return_type)?,
        is_cstyle_variadic: false,
        is_foreign: true,
        is_exposed: true,
    });

    Ok(())
}

fn lower_type(ast_type: &ast::Type) -> Result<ir::Type, CompilerError> {
    use ast::{IntegerBits::*, IntegerSign::*};
    match ast_type {
        ast::Type::Integer { bits, sign } => Ok(match (bits, sign) {
            (Normal, Signed) => ir::Type::S64,
            (Normal, Unsigned) => ir::Type::U64,
            (Bits8, Signed) => ir::Type::S8,
            (Bits8, Unsigned) => ir::Type::U8,
            (Bits16, Signed) => ir::Type::S16,
            (Bits16, Unsigned) => ir::Type::U16,
            (Bits32, Signed) => ir::Type::S32,
            (Bits32, Unsigned) => ir::Type::U32,
            (Bits64, Signed) => ir::Type::S64,
            (Bits64, Unsigned) => ir::Type::U64,
        }),
        ast::Type::Pointer(inner) => Ok(ir::Type::Pointer(Box::new(lower_type(inner)?))),
        ast::Type::Void => Ok(ir::Type::Void),
    }
}

fn lower_expression(
    builder: &mut Builder,
    expression: &Expression,
) -> Result<ir::Value, CompilerError> {
    match expression {
        Expression::Integer(value) => {
            if let Ok(value) = value.try_into() {
                Ok(ir::Value::Literal(Literal::Signed64(value)))
            } else {
                Err(CompilerError::during_lower(
                    "Integer literal does not fit within signed 64-bit integer",
                ))
            }
        }
    }
}
