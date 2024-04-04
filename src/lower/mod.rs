mod builder;

use crate::{
    error::CompilerError,
    ir::{self, BasicBlocks, Global, Literal, OverflowOperator, Value, ValueReference},
    resolved::{
        self, Destination, DestinationKind, Expression, ExpressionKind, FloatOrInteger, FloatSize,
        IntegerBits, NumericMode, StatementKind,
    },
};
use builder::Builder;

pub fn lower(ast: &resolved::Ast) -> Result<ir::Module, CompilerError> {
    let mut ir_module = ir::Module::new();

    for (structure_ref, structure) in ast.structures.iter() {
        lower_structure(&mut ir_module, structure_ref, structure)?;
    }

    for (global_ref, global) in ast.globals.iter() {
        lower_global(&mut ir_module, global_ref, global)?;
    }

    for (function_ref, function) in ast.functions.iter() {
        lower_function(&mut ir_module, function_ref, function)?;
    }

    Ok(ir_module)
}

fn lower_structure(
    ir_module: &mut ir::Module,
    structure_ref: resolved::StructureRef,
    structure: &resolved::Structure,
) -> Result<(), CompilerError> {
    let mut fields = Vec::with_capacity(structure.fields.len());

    for field in structure.fields.values() {
        fields.push(lower_type(&field.resolved_type)?);
    }

    ir_module.structures.insert(
        structure_ref,
        ir::Structure {
            fields,
            is_packed: structure.is_packed,
        },
    );

    Ok(())
}

fn lower_global(
    ir_module: &mut ir::Module,
    global_ref: resolved::GlobalRef,
    global: &resolved::Global,
) -> Result<(), CompilerError> {
    ir_module.globals.insert(
        global_ref,
        Global {
            mangled_name: global.name.to_string(),
            ir_type: lower_type(&global.resolved_type)?,
            is_foreign: global.is_foreign,
            is_thread_local: global.is_thread_local,
        },
    );

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
        let parameter_variables = function
            .variables
            .instances
            .iter()
            .take(function.variables.num_parameters)
            .map(|instance| {
                Ok(builder.push(ir::Instruction::Alloca(lower_type(
                    &instance.resolved_type,
                )?)))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Allocate non-parameter stack variables
        for variable_instance in function
            .variables
            .instances
            .iter()
            .skip(function.variables.num_parameters)
        {
            builder.push(ir::Instruction::Alloca(lower_type(
                &variable_instance.resolved_type,
            )?));
        }

        for (i, destination) in parameter_variables.into_iter().enumerate() {
            let source = builder.push(ir::Instruction::Parameter(i.try_into().unwrap()));

            builder.push(ir::Instruction::Store(ir::Store {
                source,
                destination,
            }));
        }

        lower_statements(&mut builder, ir_module, &function.statements, function)?;

        if !builder.is_block_terminated() {
            if let resolved::Type::Void = function.return_type {
                if function.name == "main" && !builder.is_block_terminated() {
                    builder.push(ir::Instruction::Return(Some(ir::Value::Literal(
                        Literal::Signed32(0),
                    ))));
                } else {
                    builder.terminate();
                }
            } else {
                return Err(CompilerError::during_lower(format!(
                    "Must return a value of type '{}' before exiting function '{}'",
                    function.return_type, function.name
                )));
            }
        }

        builder.build()
    } else {
        BasicBlocks::default()
    };

    let mut parameters = vec![];
    for parameter in function.parameters.required.iter() {
        parameters.push(lower_type(&parameter.resolved_type)?);
    }

    let mut return_type = lower_type(&function.return_type)?;

    if function.name == "main" {
        if let ir::Type::Void = return_type {
            return_type = ir::Type::S32;
        }
    }

    ir_module.functions.insert(
        function_ref,
        ir::Function {
            mangled_name: function.name.clone(),
            basicblocks,
            parameters,
            return_type,
            is_cstyle_variadic: function.parameters.is_cstyle_vararg,
            is_foreign: true,
            is_exposed: true,
            variables: vec![],
        },
    );

    Ok(())
}

fn lower_statements(
    builder: &mut Builder,
    ir_module: &ir::Module,
    statements: &[resolved::Statement],
    function: &resolved::Function,
) -> Result<Value, CompilerError> {
    let mut result = Value::Literal(Literal::Void);

    for statement in statements.iter() {
        result = match &statement.kind {
            StatementKind::Return(expression) => {
                let instruction = ir::Instruction::Return(if let Some(expression) = expression {
                    Some(lower_expression(builder, ir_module, expression, function)?)
                } else if function.name == "main" {
                    Some(ir::Value::Literal(Literal::Signed32(0)))
                } else {
                    None
                });

                builder.push(instruction);
                Value::Literal(Literal::Void)
            }
            StatementKind::Expression(expression) => {
                lower_expression(builder, ir_module, &expression.expression, function)?
            }
            StatementKind::Declaration(declaration) => {
                let destination = Value::Reference(ValueReference {
                    basicblock_id: 0,
                    instruction_id: declaration.key.index,
                });

                if let Some(value) = &declaration.value {
                    let source = lower_expression(builder, ir_module, value, function)?;

                    builder.push(ir::Instruction::Store(ir::Store {
                        source,
                        destination,
                    }));
                }

                Value::Literal(Literal::Void)
            }
            StatementKind::Assignment(assignment) => {
                let destination = lower_destination(builder, ir_module, &assignment.destination)?;
                let source = lower_expression(builder, ir_module, &assignment.value, function)?;

                builder.push(ir::Instruction::Store(ir::Store {
                    source,
                    destination,
                }));

                Value::Literal(Literal::Void)
            }
        }
    }

    Ok(result)
}

fn lower_type(resolved_type: &resolved::Type) -> Result<ir::Type, CompilerError> {
    use resolved::{IntegerBits as Bits, IntegerSign as Sign};

    match resolved_type {
        resolved::Type::Boolean => Ok(ir::Type::Boolean),
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
        resolved::Type::FloatLiteral(value) => Err(CompilerError::during_lower(format!(
            "Cannot lower unspecialized float literal {}",
            value
        ))),
        resolved::Type::Float(size) => Ok(match size {
            FloatSize::Normal => ir::Type::F64,
            FloatSize::Bits32 => ir::Type::F32,
            FloatSize::Bits64 => ir::Type::F64,
        }),
        resolved::Type::Pointer(inner) => Ok(ir::Type::Pointer(Box::new(lower_type(inner)?))),
        resolved::Type::Void => Ok(ir::Type::Void),
        resolved::Type::ManagedStructure(_, structure_ref) => {
            Ok(ir::Type::Structure(*structure_ref).reference_counted_pointer())
        }
        resolved::Type::PlainOldData(_, structure_ref) => Ok(ir::Type::Structure(*structure_ref)),
    }
}

fn lower_destination(
    builder: &mut Builder,
    ir_module: &ir::Module,
    destination: &Destination,
) -> Result<ir::Value, CompilerError> {
    match &destination.kind {
        DestinationKind::Variable(variable) => {
            let pointer = Value::Reference(ValueReference {
                basicblock_id: 0,
                instruction_id: variable.key.index,
            });

            Ok(pointer)
        }
        DestinationKind::GlobalVariable(global_variable) => {
            let pointer = builder.push(ir::Instruction::GlobalVariable(global_variable.reference));
            Ok(pointer)
        }
        DestinationKind::Member(destination, structure_ref, index, _, memory_management) => {
            let subject_pointer = lower_destination(builder, ir_module, destination)?;

            let subject_pointer = match memory_management {
                resolved::MemoryManagement::None => subject_pointer,
                resolved::MemoryManagement::ReferenceCounted => {
                    // Load pointer from pointed object

                    let container =
                        ir::Type::Structure(*structure_ref).reference_counted_no_pointer();

                    let subject_pointer = builder.push(ir::Instruction::Load((
                        subject_pointer,
                        container.pointer(),
                    )));

                    builder.push(ir::Instruction::Member(subject_pointer, container, 1))
                }
            };

            Ok(builder.push(ir::Instruction::Member(
                subject_pointer,
                ir::Type::Structure(*structure_ref),
                *index,
            )))
        }
    }
}

fn lower_expression(
    builder: &mut Builder,
    ir_module: &ir::Module,
    expression: &Expression,
    function: &resolved::Function,
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
        ExpressionKind::Float(size, value) => Ok(Value::Literal(match size {
            FloatSize::Bits32 => Literal::Float32(*value as f32),
            FloatSize::Bits64 | FloatSize::Normal => Literal::Float64(*value),
        })),
        ExpressionKind::NullTerminatedString(value) => Ok(ir::Value::Literal(
            Literal::NullTerminatedString(value.clone()),
        )),
        ExpressionKind::String(_value) => {
            unimplemented!(
                "String literals are not fully implemented yet, still need ability to lower"
            )
        }
        ExpressionKind::Call(call) => {
            let mut arguments = vec![];

            for argument in call.arguments.iter() {
                arguments.push(lower_expression(builder, ir_module, argument, function)?);
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
            let initial_value =
                lower_expression(builder, ir_module, &declare_assign.value, function)?;

            let destination = Value::Reference(ValueReference {
                basicblock_id: 0,
                instruction_id: declare_assign.key.index,
            });

            builder.push(ir::Instruction::Store(ir::Store {
                source: initial_value,
                destination: destination.clone(),
            }));

            let ir_type = lower_type(&declare_assign.resolved_type)?;
            Ok(builder.push(ir::Instruction::Load((destination, ir_type))))
        }
        ExpressionKind::BinaryOperation(binary_operation) => {
            let left = lower_expression(
                builder,
                ir_module,
                &binary_operation.left.expression,
                function,
            )?;
            let right = lower_expression(
                builder,
                ir_module,
                &binary_operation.right.expression,
                function,
            )?;
            let operands = ir::BinaryOperands::new(left, right);

            match &binary_operation.operator {
                resolved::BinaryOperator::Add(mode) => Ok(builder.push(match mode {
                    NumericMode::Integer(_) => {
                        ir::Instruction::Add(operands, FloatOrInteger::Integer)
                    }
                    NumericMode::Float => ir::Instruction::Add(operands, FloatOrInteger::Float),
                    NumericMode::CheckOverflow(sign) => ir::Instruction::Checked(
                        ir::OverflowOperation {
                            operator: OverflowOperator::Add,
                            bits: IntegerBits::Normal,
                            sign: *sign,
                        },
                        operands,
                    ),
                })),
                resolved::BinaryOperator::Subtract(mode) => Ok(builder.push(match mode {
                    NumericMode::Integer(_) => {
                        ir::Instruction::Subtract(operands, FloatOrInteger::Integer)
                    }
                    NumericMode::Float => {
                        ir::Instruction::Subtract(operands, FloatOrInteger::Float)
                    }
                    NumericMode::CheckOverflow(sign) => ir::Instruction::Checked(
                        ir::OverflowOperation {
                            operator: OverflowOperator::Subtract,
                            bits: IntegerBits::Normal,
                            sign: *sign,
                        },
                        operands,
                    ),
                })),
                resolved::BinaryOperator::Multiply(mode) => Ok(builder.push(match mode {
                    NumericMode::Integer(_) => {
                        ir::Instruction::Multiply(operands, FloatOrInteger::Integer)
                    }
                    NumericMode::Float => {
                        ir::Instruction::Multiply(operands, FloatOrInteger::Float)
                    }
                    NumericMode::CheckOverflow(sign) => ir::Instruction::Checked(
                        ir::OverflowOperation {
                            operator: OverflowOperator::Multiply,
                            bits: IntegerBits::Normal,
                            sign: *sign,
                        },
                        operands,
                    ),
                })),
                resolved::BinaryOperator::Divide(mode) => {
                    Ok(builder.push(ir::Instruction::Divide(operands, *mode)))
                }
                resolved::BinaryOperator::Modulus(mode) => {
                    Ok(builder.push(ir::Instruction::Modulus(operands, *mode)))
                }
                resolved::BinaryOperator::Equals(mode) => {
                    Ok(builder.push(ir::Instruction::Equals(operands, *mode)))
                }
                resolved::BinaryOperator::NotEquals(mode) => {
                    Ok(builder.push(ir::Instruction::NotEquals(operands, *mode)))
                }
                resolved::BinaryOperator::LessThan(mode) => {
                    Ok(builder.push(ir::Instruction::LessThan(operands, *mode)))
                }
                resolved::BinaryOperator::LessThanEq(mode) => {
                    Ok(builder.push(ir::Instruction::LessThanEq(operands, *mode)))
                }
                resolved::BinaryOperator::GreaterThan(mode) => {
                    Ok(builder.push(ir::Instruction::GreaterThan(operands, *mode)))
                }
                resolved::BinaryOperator::GreaterThanEq(mode) => {
                    Ok(builder.push(ir::Instruction::GreaterThanEq(operands, *mode)))
                }
                resolved::BinaryOperator::BitwiseAnd => {
                    Ok(builder.push(ir::Instruction::BitwiseAnd(operands)))
                }
                resolved::BinaryOperator::BitwiseOr => {
                    Ok(builder.push(ir::Instruction::BitwiseOr(operands)))
                }
                resolved::BinaryOperator::BitwiseXor => {
                    Ok(builder.push(ir::Instruction::BitwiseXor(operands)))
                }
                resolved::BinaryOperator::LogicalLeftShift
                | resolved::BinaryOperator::LeftShift => {
                    Ok(builder.push(ir::Instruction::LeftShift(operands)))
                }
                resolved::BinaryOperator::RightShift => {
                    Ok(builder.push(ir::Instruction::RightShift(operands)))
                }
                resolved::BinaryOperator::LogicalRightShift => {
                    Ok(builder.push(ir::Instruction::LogicalRightShift(operands)))
                }
            }
        }
        ExpressionKind::IntegerExtend(value, resolved_type) => {
            let value = lower_expression(builder, ir_module, value, function)?;
            let ir_type = lower_type(resolved_type)?;

            Ok(builder.push(
                match resolved_type
                    .sign()
                    .expect("integer extend result type to be an integer type")
                {
                    resolved::IntegerSign::Signed => ir::Instruction::SignExtend(value, ir_type),
                    resolved::IntegerSign::Unsigned => ir::Instruction::ZeroExtend(value, ir_type),
                },
            ))
        }
        ExpressionKind::FloatExtend(value, resolved_type) => {
            let value = lower_expression(builder, ir_module, value, function)?;
            let ir_type = lower_type(resolved_type)?;
            Ok(builder.push(ir::Instruction::FloatExtend(value, ir_type)))
        }
        ExpressionKind::Member(
            subject_destination,
            structure_ref,
            index,
            resolved_type,
            memory_management,
        ) => {
            let subject_pointer = lower_destination(builder, ir_module, subject_destination)?;

            let subject_pointer = match memory_management {
                resolved::MemoryManagement::None => subject_pointer,
                resolved::MemoryManagement::ReferenceCounted => {
                    // Load pointer from pointed object

                    let container =
                        ir::Type::Structure(*structure_ref).reference_counted_no_pointer();

                    let subject_pointer = builder.push(ir::Instruction::Load((
                        subject_pointer,
                        container.pointer(),
                    )));

                    builder.push(ir::Instruction::Member(subject_pointer, container, 1))
                }
            };

            let member = builder.push(ir::Instruction::Member(
                subject_pointer,
                ir::Type::Structure(*structure_ref),
                *index,
            ));

            let ir_type = lower_type(resolved_type)?;
            Ok(builder.push(ir::Instruction::Load((member, ir_type))))
        }
        ExpressionKind::StructureLiteral {
            structure_type,
            fields,
            memory_management,
        } => {
            let ir_type = lower_type(structure_type)?;
            let mut values = Vec::with_capacity(fields.len());

            // Evaluate field values in the order specified by the struct literal
            for (expression, index) in fields.values() {
                let ir_value = lower_expression(builder, ir_module, expression, function)?;
                values.push((index, ir_value));
            }

            // Sort resulting values by index
            values.sort_by(|(a, _), (b, _)| a.cmp(b));

            // Drop the index part of the values
            let values = values.drain(..).map(|(_, value)| value).collect();

            match memory_management {
                resolved::MemoryManagement::None => {
                    Ok(builder.push(ir::Instruction::StructureLiteral(ir_type, values)))
                }
                resolved::MemoryManagement::ReferenceCounted => {
                    let flat =
                        builder.push(ir::Instruction::StructureLiteral(ir_type.clone(), values));

                    let container = ir_type.reference_counted_no_pointer();
                    let heap_memory = builder.push(ir::Instruction::Malloc(container.clone()));

                    // TODO: Assert that malloc didn't return NULL

                    let at_reference_count = builder.push(ir::Instruction::Member(
                        heap_memory.clone(),
                        container.clone(),
                        0,
                    ));

                    let at_value = builder.push(ir::Instruction::Member(
                        heap_memory.clone(),
                        container.clone(),
                        1,
                    ));

                    builder.push(ir::Instruction::Store(ir::Store {
                        source: flat.clone(),
                        destination: at_reference_count,
                    }));

                    builder.push(ir::Instruction::Store(ir::Store {
                        source: flat,
                        destination: at_value,
                    }));

                    Ok(heap_memory)
                }
            }
        }
        ExpressionKind::UnaryOperator(unary_operation) => {
            let inner = lower_expression(
                builder,
                ir_module,
                &unary_operation.inner.expression,
                function,
            )?;

            Ok(builder.push(match unary_operation.operator {
                resolved::UnaryOperator::Not => ir::Instruction::IsZero(inner),
                resolved::UnaryOperator::BitComplement => ir::Instruction::BitComplement(inner),
                resolved::UnaryOperator::Negate => ir::Instruction::Negate(inner),
            }))
        }
        ExpressionKind::Conditional(conditional) => {
            let resume_basicblock_id = builder.new_block();

            let mut incoming = vec![];

            for resolved::Branch { condition, block } in conditional.branches.iter() {
                let expression = &condition.expression;
                let condition = lower_expression(builder, ir_module, expression, function)?;

                let true_basicblock_id = builder.new_block();
                let false_basicblock_id = builder.new_block();

                builder.push(ir::Instruction::ConditionalBreak(
                    condition,
                    ir::ConditionalBreak {
                        true_basicblock_id,
                        false_basicblock_id,
                    },
                ));

                builder.use_block(true_basicblock_id);
                let value = lower_statements(builder, ir_module, &block.statements, function)?;

                incoming.push(ir::PhiIncoming {
                    basicblock_id: builder.current_block_id(),
                    value,
                });
                builder.continues_to(resume_basicblock_id);

                builder.use_block(false_basicblock_id);
            }

            if let Some(block) = &conditional.otherwise {
                let value = lower_statements(builder, ir_module, &block.statements, function)?;
                incoming.push(ir::PhiIncoming {
                    basicblock_id: builder.current_block_id(),
                    value,
                });
            }

            builder.continues_to(resume_basicblock_id);
            builder.use_block(resume_basicblock_id);

            if conditional.otherwise.is_some() {
                let ir_type = lower_type(&conditional.result_type)?;
                Ok(builder.push(ir::Instruction::Phi(ir::Phi { ir_type, incoming })))
            } else {
                Ok(Value::Literal(Literal::Void))
            }
        }
        ExpressionKind::BooleanLiteral(value) => Ok(Value::Literal(Literal::Boolean(*value))),
        ExpressionKind::While(while_loop) => {
            let evaluate_basicblock_id = builder.new_block();
            let true_basicblock_id = builder.new_block();
            let false_basicblock_id = builder.new_block();

            builder.continues_to(evaluate_basicblock_id);
            builder.use_block(evaluate_basicblock_id);

            let condition = lower_expression(builder, ir_module, &while_loop.condition, function)?;

            builder.push(ir::Instruction::ConditionalBreak(
                condition,
                ir::ConditionalBreak {
                    true_basicblock_id,
                    false_basicblock_id,
                },
            ));

            builder.use_block(true_basicblock_id);
            lower_statements(builder, ir_module, &while_loop.block.statements, function)?;
            builder.continues_to(evaluate_basicblock_id);

            builder.use_block(false_basicblock_id);
            Ok(Value::Literal(Literal::Void))
        }
    }
}
