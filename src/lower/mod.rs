mod builder;

use crate::{
    error::CompilerError,
    ir::{self, BasicBlocks, Global, Literal, Value, ValueReference},
    resolved::{self, Expression, ExpressionKind, IntegerSign, StatementKind},
};
use builder::Builder;
use std::ffi::CString;

pub fn lower(ast: &resolved::Ast) -> Result<ir::Module, CompilerError> {
    let mut ir_module = ir::Module::new();

    for (global_ref, global) in ast.globals.iter() {
        lower_global(&mut ir_module, global_ref, global)?;
    }

    for (function_ref, function) in ast.functions.iter() {
        lower_function(&mut ir_module, function_ref, function)?;
    }

    Ok(ir_module)
}

fn lower_global(ir_module: &mut ir::Module, global_ref: resolved::GlobalRef, global: &resolved::Global) -> Result<(), CompilerError> {
    ir_module.globals.insert(global_ref, Global {
        mangled_name: global.name.to_string(),
        ir_type: lower_type(&global.resolved_type)?,
        is_foreign: global.is_foreign,
        is_thread_local: global.is_thread_local,
    });

    Ok(())
}

fn lower_function(
    ir_module: &mut ir::Module,
    function_ref: resolved::FunctionRef,
    function: &resolved::Function,
) -> Result<(), CompilerError> {
    let basicblocks = if !function.is_foreign {
        let mut builder = Builder::new();

        // Allocate parameters
        for (i, variable_type) in function
            .variables
            .types
            .iter()
            .enumerate()
            .take(function.variables.num_parameters)
        {
            let destination = builder.push(ir::Instruction::Alloca(lower_type(variable_type)?));
            let source = builder.push(ir::Instruction::Parameter(i.try_into().unwrap()));

            builder.push(ir::Instruction::Store(ir::Store {
                source,
                destination,
            }));
        }

        // Allocate non-parameter stack variables
        for variable_type in function
            .variables
            .types
            .iter()
            .skip(function.variables.num_parameters)
        {
            builder.push(ir::Instruction::Alloca(lower_type(variable_type)?));
        }

        for statement in function.statements.iter() {
            match &statement.kind {
                StatementKind::Return(expression) => {
                    let instruction =
                        ir::Instruction::Return(if let Some(expression) = expression {
                            Some(lower_expression(&mut builder, ir_module, expression)?)
                        } else {
                            None
                        });

                    builder.push(instruction);
                }
                StatementKind::Expression(expression) => {
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
            variables: vec![],
        },
    );

    Ok(())
}

fn lower_type(resolved_type: &resolved::Type) -> Result<ir::Type, CompilerError> {
    use resolved::{IntegerBits as Bits, IntegerSign as Sign};

    match resolved_type {
        resolved::Type::Integer { bits, sign } => Ok(match (bits, sign) {
            (Bits::Normal, Sign::Signed) => ir::Type::S64,
            (Bits::Normal, Sign::Unsigned) => ir::Type::U64,
            (Bits::Bits8, Sign::Signed) => ir::Type::S8,
            (Bits::Bits8, Sign::Unsigned) => ir::Type::U8,
            (Bits::Bits16, Sign::Signed) => ir::Type::S16,
            (Bits::Bits16, Sign::Unsigned) => ir::Type::U16,
            (Bits::Bits32, Sign::Signed) => ir::Type::S32,
            (Bits::Bits32, Sign::Unsigned) => ir::Type::U32,
            (Bits::Bits64, Sign::Signed) => ir::Type::S64,
            (Bits::Bits64, Sign::Unsigned) => ir::Type::U64,
        }),
        resolved::Type::IntegerLiteral(value) => Err(CompilerError::during_lower(format!(
            "Cannot lower unspecialized integer literal {}",
            value
        ))),
        resolved::Type::Pointer(inner) => Ok(ir::Type::Pointer(Box::new(lower_type(inner)?))),
        resolved::Type::Void => Ok(ir::Type::Void),
    }
}

fn lower_expression(
    builder: &mut Builder,
    ir_module: &ir::Module,
    expression: &Expression,
) -> Result<ir::Value, CompilerError> {
    match &expression.kind {
        ExpressionKind::IntegerLiteral(value) => Err(CompilerError::during_lower(format!(
            "Cannot lower unspecialized integer literal {}",
            value
        ))),
        ExpressionKind::Integer { value, bits, sign } => {
            use resolved::{IntegerLiteralBits as Bits, IntegerSign as Sign};

            match (bits, sign) {
                (Bits::Bits8, Sign::Signed) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Signed8(value)))
                    } else {
                        Err(CompilerError::during_lower(format!(
                            "Cannot fit {} into int8",
                            value
                        )))
                    }
                }
                (Bits::Bits8, Sign::Unsigned) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Unsigned8(value)))
                    } else {
                        Err(CompilerError::during_lower(format!(
                            "Cannot fit {} into uint8",
                            value
                        )))
                    }
                }
                (Bits::Bits16, Sign::Signed) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Signed16(value)))
                    } else {
                        Err(CompilerError::during_lower(format!(
                            "Cannot fit {} into int16",
                            value
                        )))
                    }
                }
                (Bits::Bits16, Sign::Unsigned) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Unsigned16(value)))
                    } else {
                        Err(CompilerError::during_lower(format!(
                            "Cannot fit {} into uint16",
                            value
                        )))
                    }
                }
                (Bits::Bits32, Sign::Signed) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Signed32(value)))
                    } else {
                        Err(CompilerError::during_lower(format!(
                            "Cannot fit {} into int32",
                            value
                        )))
                    }
                }
                (Bits::Bits32, Sign::Unsigned) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Unsigned32(value)))
                    } else {
                        Err(CompilerError::during_lower(format!(
                            "Cannot fit {} into uint32",
                            value
                        )))
                    }
                }
                (Bits::Bits64, Sign::Signed) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Signed64(value)))
                    } else {
                        Err(CompilerError::during_lower(format!(
                            "Cannot fit {} into int64",
                            value
                        )))
                    }
                }
                (Bits::Bits64, Sign::Unsigned) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Unsigned64(value)))
                    } else {
                        Err(CompilerError::during_lower(format!(
                            "Cannot fit {} into uint64",
                            value
                        )))
                    }
                }
            }
        }
        ExpressionKind::NullTerminatedString(value) => Ok(ir::Value::Literal(
            Literal::NullTerminatedString(value.clone()),
        )),
        ExpressionKind::Call(call) => {
            let mut arguments = vec![];

            for argument in call.arguments.iter() {
                arguments.push(lower_expression(builder, ir_module, argument)?);
            }

            Ok(builder.push(ir::Instruction::Call(ir::Call {
                function: call.function,
                arguments,
            })))
        }
        ExpressionKind::Variable(variable) => {
            let pointer = Value::Reference(ValueReference {
                basicblock_id: 0,
                instruction_id: variable.key.index,
            });

            let ir_type = lower_type(&variable.resolved_type)?;

            Ok(builder.push(ir::Instruction::Load((pointer, ir_type))))
        }
        ExpressionKind::GlobalVariable(global_variable) => {
            let pointer = builder.push(ir::Instruction::GlobalVariable(global_variable.reference));
            let ir_type = lower_type(&global_variable.resolved_type)?;
            Ok(builder.push(ir::Instruction::Load((pointer, ir_type))))
        }
        ExpressionKind::DeclareAssign(declare_assign) => {
            let initial_value = lower_expression(builder, ir_module, &declare_assign.value)?;

            let destination = Value::Reference(ValueReference {
                basicblock_id: 0,
                instruction_id: declare_assign.key.index,
            });

            builder.push(ir::Instruction::Store(ir::Store {
                source: initial_value,
                destination: destination.clone(),
            }));

            Ok(destination)
        }
        ExpressionKind::BinaryOperation(binary_operation) => {
            let left = lower_expression(builder, ir_module, &binary_operation.left.expression)?;
            let right = lower_expression(builder, ir_module, &binary_operation.right.expression)?;
            let operands = ir::BinaryOperands::new(left, right);

            match binary_operation.operator {
                resolved::BinaryOperator::Add => Ok(builder.push(ir::Instruction::Add(operands))),
                resolved::BinaryOperator::Subtract => {
                    Ok(builder.push(ir::Instruction::Subtract(operands)))
                }
                resolved::BinaryOperator::Multiply => {
                    Ok(builder.push(ir::Instruction::Multiply(operands)))
                }
                resolved::BinaryOperator::Divide => {
                    match binary_operation.left.resolved_type.sign() {
                        Some(IntegerSign::Signed) => {
                            Ok(builder.push(ir::Instruction::DivideSigned(operands)))
                        }
                        Some(IntegerSign::Unsigned) => {
                            Ok(builder.push(ir::Instruction::DivideUnsigned(operands)))
                        }
                        None => Err(CompilerError::during_lower("Cannot divide non-integer")),
                    }
                }
                resolved::BinaryOperator::Modulus => {
                    match binary_operation.left.resolved_type.sign() {
                        Some(IntegerSign::Signed) => {
                            Ok(builder.push(ir::Instruction::ModulusSigned(operands)))
                        }
                        Some(IntegerSign::Unsigned) => {
                            Ok(builder.push(ir::Instruction::ModulusUnsigned(operands)))
                        }
                        None => Err(CompilerError::during_lower("Cannot modulo non-integer")),
                    }
                }
            }
        }
    }
}
