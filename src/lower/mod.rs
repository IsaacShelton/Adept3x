mod builder;

use crate::{
    error::CompilerError,
    ir::{self, BasicBlocks, Literal},
    resolved::{self, Expression, Statement},
};
use builder::Builder;
use std::ffi::CString;

pub fn lower(ast: &resolved::Ast) -> Result<ir::Module, CompilerError> {
    let mut ir_module = ir::Module::new();

    for (function_ref, function) in ast.functions.iter() {
        lower_function(&mut ir_module, function_ref, function)?;
    }

    Ok(ir_module)
}

fn lower_function(
    ir_module: &mut ir::Module,
    function_ref: resolved::FunctionRef,
    function: &resolved::Function,
) -> Result<(), CompilerError> {
    let basicblocks = if !function.is_foreign {
        let mut builder = Builder::new();

        for statement in function.statements.iter() {
            match statement {
                Statement::Return(expression) => {
                    let instruction =
                        ir::Instruction::Return(if let Some(expression) = expression {
                            Some(lower_expression(&mut builder, ir_module, expression)?)
                        } else {
                            None
                        });

                    builder.push(instruction);
                }
                Statement::Expression(expression) => {
                    lower_expression(&mut builder, ir_module, expression)?;
                }
            }
        }

        builder.terminate();
        builder.build()
    } else {
        BasicBlocks::default()
    };

    let mut parameters = vec![];
    for parameter in function.parameters.required.iter() {
        parameters.push(lower_type(&parameter.resolved_type)?);
    }

    ir_module.functions.insert(
        function_ref,
        ir::Function {
            mangled_name: function.name.clone(),
            basicblocks,
            parameters,
            return_type: lower_type(&function.return_type)?,
            is_cstyle_variadic: function.parameters.is_cstyle_vararg,
            is_foreign: true,
            is_exposed: true,
        },
    );

    Ok(())
}

fn lower_type(resolved_type: &resolved::Type) -> Result<ir::Type, CompilerError> {
    use resolved::{IntegerBits::*, IntegerSign::*};

    match resolved_type {
        resolved::Type::Integer { bits, sign } => Ok(match (bits, sign) {
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
        resolved::Type::Pointer(inner) => Ok(ir::Type::Pointer(Box::new(lower_type(inner)?))),
        resolved::Type::Void => Ok(ir::Type::Void),
    }
}

fn lower_expression(
    builder: &mut Builder,
    ir_module: &ir::Module,
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
        Expression::NullTerminatedString(value) => Ok(ir::Value::Literal(
            Literal::NullTerminatedString(value.clone()),
        )),
        Expression::Call(call) => {
            let mut arguments = vec![];

            for argument in call.arguments.iter() {
                arguments.push(lower_expression(builder, ir_module, argument)?);
            }

            Ok(builder.push(ir::Instruction::Call(ir::Call {
                function: call.function,
                arguments,
            })))
        }
        Expression::Variable(name) => todo!(),
    }
}
