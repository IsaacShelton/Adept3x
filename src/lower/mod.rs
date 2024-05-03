mod builder;

use crate::{
    error::CompilerError,
    ir::{self, BasicBlocks, Global, Literal, OverflowOperator, Value, ValueReference},
    resolved::{
        self, Destination, DestinationKind, Expr, ExprKind, FloatOrInteger, FloatSize, IntegerBits,
        NumericMode, StmtKind, VariableStorageKey,
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
                new_value: source,
                destination,
            }));
        }

        lower_stmts(&mut builder, ir_module, &function.stmts, function)?;

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

fn lower_stmts(
    builder: &mut Builder,
    ir_module: &ir::Module,
    stmts: &[resolved::Stmt],
    function: &resolved::Function,
) -> Result<Value, CompilerError> {
    let mut result = Value::Literal(Literal::Void);

    for stmt in stmts.iter() {
        result = match &stmt.kind {
            StmtKind::Return(expr, drops) => {
                for variable_key in drops.drops.iter() {
                    lower_drop(builder, *variable_key, function)?;
                }

                let instruction = ir::Instruction::Return(if let Some(expr) = expr {
                    Some(lower_expr(builder, ir_module, expr, function)?)
                } else if function.name == "main" {
                    Some(ir::Value::Literal(Literal::Signed32(0)))
                } else {
                    None
                });

                builder.push(instruction);
                Value::Literal(Literal::Void)
            }
            StmtKind::Expr(expr) => lower_expr(builder, ir_module, &expr.expr, function)?,
            StmtKind::Declaration(declaration) => {
                let destination = Value::Reference(ValueReference {
                    basicblock_id: 0,
                    instruction_id: declaration.key.index,
                });

                if let Some(value) = &declaration.value {
                    let source = lower_expr(builder, ir_module, value, function)?;

                    builder.push(ir::Instruction::Store(ir::Store {
                        new_value: source,
                        destination,
                    }));
                }

                Value::Literal(Literal::Void)
            }
            StmtKind::Assignment(assignment) => {
                let destination =
                    lower_destination(builder, ir_module, &assignment.destination, function)?;

                let new_value = lower_expr(builder, ir_module, &assignment.value, function)?;

                let new_value = if let Some(operator) = &assignment.operator {
                    let destination_type = lower_type(&assignment.destination.resolved_type)?;

                    let existing_value = builder.push(ir::Instruction::Load((
                        destination.clone(),
                        destination_type,
                    )));

                    lower_basic_binary_operation(
                        builder,
                        &operator,
                        ir::BinaryOperands::new(existing_value, new_value),
                    )?
                } else {
                    new_value
                };

                builder.push(ir::Instruction::Store(ir::Store {
                    new_value,
                    destination,
                }));

                Value::Literal(Literal::Void)
            }
        };

        for variable_key in stmt.drops.iter() {
            lower_drop(builder, *variable_key, function)?;
        }
    }

    Ok(result)
}

fn lower_drop(
    builder: &mut Builder,
    variable_key: VariableStorageKey,
    function: &resolved::Function,
) -> Result<(), CompilerError> {
    let variable = function
        .variables
        .get(variable_key)
        .expect("referenced variable to exist");

    if variable.resolved_type.is_managed_structure() {
        let variable_pointer = lower_variable_to_value(variable_key);
        let variable_type = lower_type(&variable.resolved_type)?;
        let heap_pointer = builder.push(ir::Instruction::Load((variable_pointer, variable_type)));
        builder.push(ir::Instruction::Free(heap_pointer));
    }

    Ok(())
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

fn lower_variable_to_value(key: VariableStorageKey) -> Value {
    Value::Reference(ValueReference {
        basicblock_id: 0,
        instruction_id: key.index,
    })
}

fn lower_destination(
    builder: &mut Builder,
    ir_module: &ir::Module,
    destination: &Destination,
    function: &resolved::Function,
) -> Result<ir::Value, CompilerError> {
    match &destination.kind {
        DestinationKind::Variable(variable) => Ok(lower_variable_to_value(variable.key)),
        DestinationKind::GlobalVariable(global_variable) => {
            let pointer = builder.push(ir::Instruction::GlobalVariable(global_variable.reference));
            Ok(pointer)
        }
        DestinationKind::Member {
            subject,
            structure_ref,
            index,
            memory_management,
            ..
        } => {
            let subject_pointer = lower_destination(builder, ir_module, subject, function)?;

            let subject_pointer = match memory_management {
                resolved::MemoryManagement::None => subject_pointer,
                resolved::MemoryManagement::ReferenceCounted => {
                    // Load pointer from pointed object

                    let struct_type =
                        ir::Type::Structure(*structure_ref).reference_counted_no_pointer();

                    let subject_pointer = builder.push(ir::Instruction::Load((
                        subject_pointer,
                        struct_type.pointer(),
                    )));

                    builder.push(ir::Instruction::Member {
                        struct_type,
                        subject_pointer,
                        index: 1,
                    })
                }
            };

            Ok(builder.push(ir::Instruction::Member {
                subject_pointer,
                struct_type: ir::Type::Structure(*structure_ref),
                index: *index,
            }))
        }
        DestinationKind::ArrayAccess(array_access) => {
            let subject_pointer = lower_expr(builder, ir_module, &array_access.subject, function)?;
            let index = lower_expr(builder, ir_module, &array_access.index, function)?;
            let item_type = lower_type(&array_access.item_type)?;

            Ok(builder.push(ir::Instruction::ArrayAccess {
                item_type,
                subject_pointer,
                index,
            }))
        }
    }
}

fn lower_expr(
    builder: &mut Builder,
    ir_module: &ir::Module,
    expr: &Expr,
    function: &resolved::Function,
) -> Result<ir::Value, CompilerError> {
    match &expr.kind {
        ExprKind::IntegerLiteral(value) => Err(CompilerError::during_lower(format!(
            "Cannot lower unspecialized integer literal {}",
            value
        ))),
        ExprKind::Integer { value, bits, sign } => {
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
        ExprKind::Float(size, value) => Ok(Value::Literal(match size {
            FloatSize::Bits32 => Literal::Float32(*value as f32),
            FloatSize::Bits64 | FloatSize::Normal => Literal::Float64(*value),
        })),
        ExprKind::NullTerminatedString(value) => Ok(ir::Value::Literal(
            Literal::NullTerminatedString(value.clone()),
        )),
        ExprKind::String(_value) => {
            unimplemented!(
                "String literals are not fully implemented yet, still need ability to lower"
            )
        }
        ExprKind::Call(call) => {
            let mut arguments = vec![];

            for argument in call.arguments.iter() {
                arguments.push(lower_expr(builder, ir_module, argument, function)?);
            }

            Ok(builder.push(ir::Instruction::Call(ir::Call {
                function: call.function,
                arguments,
            })))
        }
        ExprKind::Variable(variable) => {
            let pointer_to_variable = lower_variable_to_value(variable.key);
            let variable_type = lower_type(&variable.resolved_type)?;
            Ok(builder.push(ir::Instruction::Load((pointer_to_variable, variable_type))))
        }
        ExprKind::GlobalVariable(global_variable) => {
            let pointer = builder.push(ir::Instruction::GlobalVariable(global_variable.reference));
            let ir_type = lower_type(&global_variable.resolved_type)?;
            Ok(builder.push(ir::Instruction::Load((pointer, ir_type))))
        }
        ExprKind::DeclareAssign(declare_assign) => {
            let initial_value = lower_expr(builder, ir_module, &declare_assign.value, function)?;

            let destination = Value::Reference(ValueReference {
                basicblock_id: 0,
                instruction_id: declare_assign.key.index,
            });

            builder.push(ir::Instruction::Store(ir::Store {
                new_value: initial_value,
                destination: destination.clone(),
            }));

            let ir_type = lower_type(&declare_assign.resolved_type)?;
            Ok(builder.push(ir::Instruction::Load((destination, ir_type))))
        }
        ExprKind::BasicBinaryOperation(operation) => {
            let left = lower_expr(builder, ir_module, &operation.left.expr, function)?;
            let right = lower_expr(builder, ir_module, &operation.right.expr, function)?;

            lower_basic_binary_operation(
                builder,
                &operation.operator,
                ir::BinaryOperands::new(left, right),
            )
        }
        ExprKind::ShortCircuitingBinaryOperation(operation) => {
            lower_short_circuiting_binary_operation(builder, ir_module, operation, function)
        }
        ExprKind::IntegerExtend(value, resolved_type) => {
            let value = lower_expr(builder, ir_module, value, function)?;
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
        ExprKind::FloatExtend(value, resolved_type) => {
            let value = lower_expr(builder, ir_module, value, function)?;
            let ir_type = lower_type(resolved_type)?;
            Ok(builder.push(ir::Instruction::FloatExtend(value, ir_type)))
        }
        ExprKind::Member {
            subject,
            structure_ref,
            index,
            field_type: resolved_field_type,
            memory_management,
        } => {
            let subject_pointer = lower_destination(builder, ir_module, subject, function)?;

            let subject_pointer = match memory_management {
                resolved::MemoryManagement::None => subject_pointer,
                resolved::MemoryManagement::ReferenceCounted => {
                    // Take off reference counted wrapper

                    // Get inner structure type
                    let struct_type =
                        ir::Type::Structure(*structure_ref).reference_counted_no_pointer();

                    // Load pointer to referece counted wrapper
                    let subject_pointer = builder.push(ir::Instruction::Load((
                        subject_pointer,
                        struct_type.pointer(),
                    )));

                    // Obtain pointer to inner data
                    builder.push(ir::Instruction::Member {
                        subject_pointer,
                        struct_type,
                        index: 1,
                    })
                }
            };

            // Access member of structure
            let member = builder.push(ir::Instruction::Member {
                subject_pointer,
                struct_type: ir::Type::Structure(*structure_ref),
                index: *index,
            });

            let ir_type = lower_type(resolved_field_type)?;
            Ok(builder.push(ir::Instruction::Load((member, ir_type))))
        }
        ExprKind::ArrayAccess(array_access) => {
            let subject = lower_expr(builder, ir_module, &array_access.subject, function)?;
            let index = lower_expr(builder, ir_module, &array_access.index, function)?;
            let item_type = lower_type(&array_access.item_type)?;

            let item = builder.push(ir::Instruction::ArrayAccess {
                item_type: item_type.clone(),
                subject_pointer: subject,
                index,
            });

            Ok(builder.push(ir::Instruction::Load((item, item_type))))
        }
        ExprKind::StructureLiteral {
            structure_type,
            fields,
            memory_management,
        } => {
            let result_ir_type = lower_type(structure_type)?;
            let mut values = Vec::with_capacity(fields.len());

            // Evaluate field values in the order specified by the struct literal
            for (expr, index) in fields.values() {
                let ir_value = lower_expr(builder, ir_module, expr, function)?;
                values.push((index, ir_value));
            }

            // Sort resulting values by index
            values.sort_by(|(a, _), (b, _)| a.cmp(b));

            // Drop the index part of the values
            let values = values.drain(..).map(|(_, value)| value).collect();

            match memory_management {
                resolved::MemoryManagement::None => {
                    Ok(builder.push(ir::Instruction::StructureLiteral(result_ir_type, values)))
                }
                resolved::MemoryManagement::ReferenceCounted => {
                    let flat = builder.push(ir::Instruction::StructureLiteral(
                        result_ir_type.clone(),
                        values,
                    ));

                    let wrapper_struct_type = result_ir_type.reference_counted_no_pointer();

                    let heap_memory =
                        builder.push(ir::Instruction::Malloc(wrapper_struct_type.clone()));

                    // TODO: Assert that malloc didn't return NULL

                    let at_reference_count = builder.push(ir::Instruction::Member {
                        subject_pointer: heap_memory.clone(),
                        struct_type: wrapper_struct_type.clone(),
                        index: 0,
                    });

                    let at_value = builder.push(ir::Instruction::Member {
                        subject_pointer: heap_memory.clone(),
                        struct_type: wrapper_struct_type.clone(),
                        index: 1,
                    });

                    builder.push(ir::Instruction::Store(ir::Store {
                        new_value: flat.clone(),
                        destination: at_reference_count,
                    }));

                    builder.push(ir::Instruction::Store(ir::Store {
                        new_value: flat,
                        destination: at_value,
                    }));

                    Ok(heap_memory)
                }
            }
        }
        ExprKind::UnaryOperation(unary_operation) => {
            let inner = lower_expr(builder, ir_module, &unary_operation.inner.expr, function)?;

            Ok(builder.push(match unary_operation.operator {
                resolved::UnaryOperator::Not => ir::Instruction::IsZero(inner),
                resolved::UnaryOperator::BitComplement => ir::Instruction::BitComplement(inner),
                resolved::UnaryOperator::Negate => ir::Instruction::Negate(inner),
            }))
        }
        ExprKind::Conditional(conditional) => {
            let resume_basicblock_id = builder.new_block();

            let mut incoming = vec![];

            for resolved::Branch { condition, block } in conditional.branches.iter() {
                let condition = lower_expr(builder, ir_module, &condition.expr, function)?;

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
                let value = lower_stmts(builder, ir_module, &block.stmts, function)?;

                incoming.push(ir::PhiIncoming {
                    basicblock_id: builder.current_block_id(),
                    value,
                });
                builder.continues_to(resume_basicblock_id);

                builder.use_block(false_basicblock_id);
            }

            if let Some(block) = &conditional.otherwise {
                let value = lower_stmts(builder, ir_module, &block.stmts, function)?;
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
        ExprKind::BooleanLiteral(value) => Ok(Value::Literal(Literal::Boolean(*value))),
        ExprKind::While(while_loop) => {
            let evaluate_basicblock_id = builder.new_block();
            let true_basicblock_id = builder.new_block();
            let false_basicblock_id = builder.new_block();

            builder.continues_to(evaluate_basicblock_id);
            builder.use_block(evaluate_basicblock_id);

            let condition = lower_expr(builder, ir_module, &while_loop.condition, function)?;

            builder.push(ir::Instruction::ConditionalBreak(
                condition,
                ir::ConditionalBreak {
                    true_basicblock_id,
                    false_basicblock_id,
                },
            ));

            builder.use_block(true_basicblock_id);
            lower_stmts(builder, ir_module, &while_loop.block.stmts, function)?;
            builder.continues_to(evaluate_basicblock_id);

            builder.use_block(false_basicblock_id);
            Ok(Value::Literal(Literal::Void))
        }
    }
}

pub fn lower_basic_binary_operation(
    builder: &mut Builder,
    operator: &resolved::BasicBinaryOperator,
    operands: ir::BinaryOperands,
) -> Result<Value, CompilerError> {
    match operator {
        resolved::BasicBinaryOperator::Add(mode) => Ok(builder.push(match mode {
            NumericMode::Integer(_) => ir::Instruction::Add(operands, FloatOrInteger::Integer),
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
        resolved::BasicBinaryOperator::Subtract(mode) => Ok(builder.push(match mode {
            NumericMode::Integer(_) => ir::Instruction::Subtract(operands, FloatOrInteger::Integer),
            NumericMode::Float => ir::Instruction::Subtract(operands, FloatOrInteger::Float),
            NumericMode::CheckOverflow(sign) => ir::Instruction::Checked(
                ir::OverflowOperation {
                    operator: OverflowOperator::Subtract,
                    bits: IntegerBits::Normal,
                    sign: *sign,
                },
                operands,
            ),
        })),
        resolved::BasicBinaryOperator::Multiply(mode) => Ok(builder.push(match mode {
            NumericMode::Integer(_) => ir::Instruction::Multiply(operands, FloatOrInteger::Integer),
            NumericMode::Float => ir::Instruction::Multiply(operands, FloatOrInteger::Float),
            NumericMode::CheckOverflow(sign) => ir::Instruction::Checked(
                ir::OverflowOperation {
                    operator: OverflowOperator::Multiply,
                    bits: IntegerBits::Normal,
                    sign: *sign,
                },
                operands,
            ),
        })),
        resolved::BasicBinaryOperator::Divide(mode) => {
            Ok(builder.push(ir::Instruction::Divide(operands, *mode)))
        }
        resolved::BasicBinaryOperator::Modulus(mode) => {
            Ok(builder.push(ir::Instruction::Modulus(operands, *mode)))
        }
        resolved::BasicBinaryOperator::Equals(mode) => {
            Ok(builder.push(ir::Instruction::Equals(operands, *mode)))
        }
        resolved::BasicBinaryOperator::NotEquals(mode) => {
            Ok(builder.push(ir::Instruction::NotEquals(operands, *mode)))
        }
        resolved::BasicBinaryOperator::LessThan(mode) => {
            Ok(builder.push(ir::Instruction::LessThan(operands, *mode)))
        }
        resolved::BasicBinaryOperator::LessThanEq(mode) => {
            Ok(builder.push(ir::Instruction::LessThanEq(operands, *mode)))
        }
        resolved::BasicBinaryOperator::GreaterThan(mode) => {
            Ok(builder.push(ir::Instruction::GreaterThan(operands, *mode)))
        }
        resolved::BasicBinaryOperator::GreaterThanEq(mode) => {
            Ok(builder.push(ir::Instruction::GreaterThanEq(operands, *mode)))
        }
        resolved::BasicBinaryOperator::BitwiseAnd => {
            Ok(builder.push(ir::Instruction::BitwiseAnd(operands)))
        }
        resolved::BasicBinaryOperator::BitwiseOr => {
            Ok(builder.push(ir::Instruction::BitwiseOr(operands)))
        }
        resolved::BasicBinaryOperator::BitwiseXor => {
            Ok(builder.push(ir::Instruction::BitwiseXor(operands)))
        }
        resolved::BasicBinaryOperator::LogicalLeftShift
        | resolved::BasicBinaryOperator::LeftShift => {
            Ok(builder.push(ir::Instruction::LeftShift(operands)))
        }
        resolved::BasicBinaryOperator::RightShift => {
            Ok(builder.push(ir::Instruction::RightShift(operands)))
        }
        resolved::BasicBinaryOperator::LogicalRightShift => {
            Ok(builder.push(ir::Instruction::LogicalRightShift(operands)))
        }
    }
}

#[derive(Debug)]
struct BinaryShortCircuit {
    pub left: Value,
    pub right: Value,
    pub left_done_block_id: usize,
    pub right_done_block_id: usize,
    pub evaluate_right_block_id: usize,
}

pub fn lower_short_circuiting_binary_operation(
    builder: &mut Builder,
    ir_module: &ir::Module,
    operation: &resolved::ShortCircuitingBinaryOperation,
    function: &resolved::Function,
) -> Result<Value, CompilerError> {
    let short_circuit = lower_pre_short_circuit(builder, ir_module, operation, function)?;
    let merge_block_id = builder.new_block();
    builder.continues_to(merge_block_id);
    builder.use_block(short_circuit.left_done_block_id);

    let (conditional_break, early_result) = match operation.operator {
        resolved::ShortCircuitingBinaryOperator::And => (
            ir::ConditionalBreak {
                true_basicblock_id: short_circuit.evaluate_right_block_id,
                false_basicblock_id: merge_block_id,
            },
            false,
        ),
        resolved::ShortCircuitingBinaryOperator::Or => (
            ir::ConditionalBreak {
                true_basicblock_id: merge_block_id,
                false_basicblock_id: short_circuit.evaluate_right_block_id,
            },
            true,
        ),
    };

    builder.push(ir::Instruction::ConditionalBreak(
        short_circuit.left,
        conditional_break,
    ));
    builder.use_block(merge_block_id);

    Ok(builder.push(ir::Instruction::Phi(ir::Phi {
        ir_type: ir::Type::Boolean,
        incoming: vec![
            ir::PhiIncoming {
                basicblock_id: short_circuit.left_done_block_id,
                value: ir::Value::Literal(Literal::Boolean(early_result)),
            },
            ir::PhiIncoming {
                basicblock_id: short_circuit.right_done_block_id,
                value: short_circuit.right,
            },
        ],
    })))
}

fn lower_pre_short_circuit(
    builder: &mut Builder,
    ir_module: &ir::Module,
    operation: &resolved::ShortCircuitingBinaryOperation,
    function: &resolved::Function,
) -> Result<BinaryShortCircuit, CompilerError> {
    let left = lower_expr(builder, ir_module, &operation.left.expr, function)?;

    let left_done_block_id = builder.current_block_id();
    let evaluate_right_block_id = builder.new_block();
    builder.use_block(evaluate_right_block_id);

    let right = lower_expr(builder, ir_module, &operation.right.expr, function)?;
    let right_done_block_id = builder.current_block_id();

    Ok(BinaryShortCircuit {
        left,
        right,
        left_done_block_id,
        right_done_block_id,
        evaluate_right_block_id,
    })
}
