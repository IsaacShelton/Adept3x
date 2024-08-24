mod builder;
mod cast;
mod error;

use self::error::{LowerError, LowerErrorKind};
use crate::{
    ast::{CInteger, IntegerBits, IntegerRigidity},
    cli::BuildOptions,
    ir::{self, BasicBlocks, Global, Literal, OverflowOperator, Value, ValueReference},
    resolved::{
        self, Destination, DestinationKind, Expr, ExprKind, FloatOrInteger, FloatSize, Member,
        NumericMode, SignOrIndeterminate, StmtKind, StructureLiteral, VariableStorageKey,
    },
    tag::Tag,
    target::{Target, TargetOsExt},
};
use builder::Builder;
use cast::{integer_cast, integer_extend, integer_truncate};
use resolved::IntegerSign;

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
        lower_function(&mut ir_module, function_ref, function, ast)?;
    }

    if options.emit_ir {
        use std::{fs::File, io::Write};
        let mut f = File::create("out.ir").expect("failed to emit ir to file");
        writeln!(&mut f, "{:#?}", ir_module).expect("failed to write ir to file");
    }

    Ok(ir_module)
}

fn lower_structure(
    ir_module: &mut ir::Module,
    structure_ref: resolved::StructureRef,
    structure: &resolved::Structure,
    resolved_ast: &resolved::Ast,
) -> Result<(), LowerError> {
    let mut fields = Vec::with_capacity(structure.fields.len());

    for field in structure.fields.values() {
        fields.push(ir::Field {
            ir_type: lower_type(&ir_module.target, &field.resolved_type, resolved_ast)?,
            properties: ir::FieldProperties::default(),
            source: field.source,
        });
    }

    ir_module.structures.insert(
        structure_ref,
        ir::Structure {
            fields,
            is_packed: structure.is_packed,
            source: structure.source,
        },
    );

    Ok(())
}

fn lower_global(
    ir_module: &mut ir::Module,
    global_ref: resolved::GlobalVarRef,
    global: &resolved::GlobalVar,
    resolved_ast: &resolved::Ast,
) -> Result<(), LowerError> {
    ir_module.globals.insert(
        global_ref,
        Global {
            mangled_name: global.name.to_string(),
            ir_type: lower_type(&ir_module.target, &global.resolved_type, resolved_ast)?,
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
    resolved_ast: &resolved::Ast,
) -> Result<(), LowerError> {
    let basicblocks = if !function.is_foreign {
        let mut builder = Builder::new_with_starting_block();

        // Allocate parameters
        let parameter_variables = function
            .variables
            .instances
            .iter()
            .take(function.variables.num_parameters)
            .map(|instance| {
                Ok(builder.push(ir::Instruction::Alloca(lower_type(
                    &ir_module.target,
                    &instance.resolved_type,
                    resolved_ast,
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
                &ir_module.target,
                &variable_instance.resolved_type,
                resolved_ast,
            )?));
        }

        for (i, destination) in parameter_variables.into_iter().enumerate() {
            let source = builder.push(ir::Instruction::Parameter(i.try_into().unwrap()));

            builder.push(ir::Instruction::Store(ir::Store {
                new_value: source,
                destination,
            }));
        }

        lower_stmts(
            &mut builder,
            ir_module,
            &function.stmts,
            function,
            resolved_ast,
        )?;

        if !builder.is_block_terminated() {
            if function.return_type.kind.is_void() {
                if function.tag == Some(Tag::Main) && !builder.is_block_terminated() {
                    builder.push(ir::Instruction::Return(Some(ir::Value::Literal(
                        Literal::Signed32(0),
                    ))));
                } else {
                    builder.terminate();
                }
            } else {
                return Err(LowerErrorKind::MustReturnValueOfTypeBeforeExitingFunction {
                    return_type: function.return_type.to_string(),
                    function: function.name.clone(),
                }
                .at(function.source));
            }
        }

        builder.build()
    } else {
        BasicBlocks::default()
    };

    let mut parameters = vec![];
    for parameter in function.parameters.required.iter() {
        parameters.push(lower_type(
            &ir_module.target,
            &parameter.resolved_type,
            resolved_ast,
        )?);
    }

    let mut return_type = lower_type(&ir_module.target, &function.return_type, resolved_ast)?;

    if function.tag == Some(Tag::Main) {
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
            abide_abi: function.abide_abi && ir_module.target.arch().is_some(),
        },
    );

    Ok(())
}

fn lower_stmts(
    builder: &mut Builder,
    ir_module: &ir::Module,
    stmts: &[resolved::Stmt],
    function: &resolved::Function,
    resolved_ast: &resolved::Ast,
) -> Result<Value, LowerError> {
    let mut result = Value::Literal(Literal::Void);

    for stmt in stmts.iter() {
        result = match &stmt.kind {
            StmtKind::Return(expr, drops) => {
                for variable_key in drops.drops.iter() {
                    lower_drop(
                        builder,
                        &ir_module.target,
                        *variable_key,
                        function,
                        resolved_ast,
                    )?;
                }

                let instruction = ir::Instruction::Return(if let Some(expr) = expr {
                    Some(lower_expr(
                        builder,
                        ir_module,
                        expr,
                        function,
                        resolved_ast,
                    )?)
                } else if function.tag == Some(Tag::Main) {
                    Some(ir::Value::Literal(Literal::Signed32(0)))
                } else {
                    None
                });

                builder.push(instruction);
                Value::Literal(Literal::Void)
            }
            StmtKind::Expr(expr) => {
                lower_expr(builder, ir_module, &expr.expr, function, resolved_ast)?
            }
            StmtKind::Declaration(declaration) => {
                let destination = Value::Reference(ValueReference {
                    basicblock_id: 0,
                    instruction_id: declaration.key.index,
                });

                if let Some(value) = &declaration.value {
                    let source = lower_expr(builder, ir_module, value, function, resolved_ast)?;

                    builder.push(ir::Instruction::Store(ir::Store {
                        new_value: source,
                        destination,
                    }));
                }

                Value::Literal(Literal::Void)
            }
            StmtKind::Assignment(assignment) => {
                let destination = lower_destination(
                    builder,
                    ir_module,
                    &assignment.destination,
                    function,
                    resolved_ast,
                )?;

                let new_value = lower_expr(
                    builder,
                    ir_module,
                    &assignment.value,
                    function,
                    resolved_ast,
                )?;

                let new_value = if let Some(operator) = &assignment.operator {
                    let destination_type = lower_type(
                        &ir_module.target,
                        &assignment.destination.resolved_type,
                        resolved_ast,
                    )?;

                    let existing_value = builder.push(ir::Instruction::Load((
                        destination.clone(),
                        destination_type,
                    )));

                    lower_basic_binary_operation(
                        builder,
                        ir_module,
                        operator,
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
            lower_drop(
                builder,
                &ir_module.target,
                *variable_key,
                function,
                resolved_ast,
            )?;
        }
    }

    Ok(result)
}

fn lower_drop(
    builder: &mut Builder,
    target: &Target,
    variable_key: VariableStorageKey,
    function: &resolved::Function,
    resolved_ast: &resolved::Ast,
) -> Result<(), LowerError> {
    let variable = function
        .variables
        .get(variable_key)
        .expect("referenced variable to exist");

    if variable.resolved_type.kind.is_managed_structure() {
        let variable_pointer = lower_variable_to_value(variable_key);
        let variable_type = lower_type(target, &variable.resolved_type, resolved_ast)?;
        let heap_pointer = builder.push(ir::Instruction::Load((variable_pointer, variable_type)));
        builder.push(ir::Instruction::Free(heap_pointer));
    }

    Ok(())
}

fn lower_type(
    target: &Target,
    resolved_type: &resolved::Type,
    resolved_ast: &resolved::Ast,
) -> Result<ir::Type, LowerError> {
    use resolved::{IntegerBits as Bits, IntegerSign as Sign};

    match &resolved_type.kind {
        resolved::TypeKind::Boolean => Ok(ir::Type::Boolean),
        resolved::TypeKind::Integer(bits, sign) => Ok(match (bits, sign) {
            (Bits::Bits8, Sign::Signed) => ir::Type::S8,
            (Bits::Bits8, Sign::Unsigned) => ir::Type::U8,
            (Bits::Bits16, Sign::Signed) => ir::Type::S16,
            (Bits::Bits16, Sign::Unsigned) => ir::Type::U16,
            (Bits::Bits32, Sign::Signed) => ir::Type::S32,
            (Bits::Bits32, Sign::Unsigned) => ir::Type::U32,
            (Bits::Bits64, Sign::Signed) => ir::Type::S64,
            (Bits::Bits64, Sign::Unsigned) => ir::Type::U64,
        }),
        resolved::TypeKind::CInteger(integer, sign) => Ok(lower_c_integer(target, *integer, *sign)),
        resolved::TypeKind::IntegerLiteral(value) => {
            Err(LowerErrorKind::CannotLowerUnspecializedIntegerLiteral {
                value: value.to_string(),
            }
            .at(resolved_type.source))
        }
        resolved::TypeKind::FloatLiteral(value) => {
            Err(LowerErrorKind::CannotLowerUnspecializedFloatLiteral {
                value: value.to_string(),
            }
            .at(resolved_type.source))
        }
        resolved::TypeKind::Floating(size) => Ok(match size {
            FloatSize::Bits32 => ir::Type::F32,
            FloatSize::Bits64 => ir::Type::F64,
        }),
        resolved::TypeKind::Pointer(inner) => Ok(ir::Type::Pointer(Box::new(lower_type(
            target,
            inner,
            resolved_ast,
        )?))),
        resolved::TypeKind::Void => Ok(ir::Type::Void),
        resolved::TypeKind::ManagedStructure(_, structure_ref) => {
            Ok(ir::Type::Structure(*structure_ref).reference_counted_pointer())
        }
        resolved::TypeKind::PlainOldData(_, structure_ref) => {
            Ok(ir::Type::Structure(*structure_ref))
        }
        resolved::TypeKind::AnonymousStruct() => {
            todo!("lower anonymous struct")
        }
        resolved::TypeKind::AnonymousUnion() => {
            todo!("lower anonymous union")
        }
        resolved::TypeKind::AnonymousEnum(anonymous_enum) => {
            lower_type(target, &anonymous_enum.resolved_type, resolved_ast)
        }
        resolved::TypeKind::FixedArray(fixed_array) => {
            let size = fixed_array.size;
            let inner = lower_type(target, &fixed_array.inner, resolved_ast)?;

            Ok(ir::Type::FixedArray(Box::new(ir::FixedArray {
                length: size,
                inner,
            })))
        }
        resolved::TypeKind::FunctionPointer(_function_pointer) => Ok(ir::Type::FunctionPointer),
        resolved::TypeKind::Enum(enum_name) => {
            let enum_definition = resolved_ast
                .enums
                .get(enum_name)
                .expect("referenced enum to exist");

            lower_type(target, &enum_definition.resolved_type, resolved_ast)
        }
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
    resolved_ast: &resolved::Ast,
) -> Result<ir::Value, LowerError> {
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
            let subject_pointer =
                lower_destination(builder, ir_module, subject, function, resolved_ast)?;

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
            let subject_pointer = lower_expr(
                builder,
                ir_module,
                &array_access.subject,
                function,
                resolved_ast,
            )?;
            let index = lower_expr(
                builder,
                ir_module,
                &array_access.index,
                function,
                resolved_ast,
            )?;
            let item_type = lower_type(&ir_module.target, &array_access.item_type, resolved_ast)?;

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
    resolved_ast: &resolved::Ast,
) -> Result<ir::Value, LowerError> {
    match &expr.kind {
        ExprKind::IntegerLiteral(value) => {
            Err(LowerErrorKind::CannotLowerUnspecializedIntegerLiteral {
                value: value.to_string(),
            }
            .at(expr.source))
        }
        ExprKind::IntegerKnown(integer) => {
            let value = &integer.value;
            let sign = &integer.sign;

            let bits = match &integer.rigidity {
                IntegerRigidity::Fixed(bits) => *bits,
                IntegerRigidity::Loose(c_integer) => {
                    IntegerBits::try_from(c_integer.bytes(&ir_module.target))
                        .expect("supported integer size")
                }
            };

            match (bits, sign) {
                (IntegerBits::Bits8, IntegerSign::Signed) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Signed8(value)))
                    } else {
                        Err("i8")
                    }
                }
                (IntegerBits::Bits8, IntegerSign::Unsigned) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Unsigned8(value)))
                    } else {
                        Err("u8")
                    }
                }
                (IntegerBits::Bits16, IntegerSign::Signed) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Signed16(value)))
                    } else {
                        Err("i16")
                    }
                }
                (IntegerBits::Bits16, IntegerSign::Unsigned) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Unsigned16(value)))
                    } else {
                        Err("u16")
                    }
                }
                (IntegerBits::Bits32, IntegerSign::Signed) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Signed32(value)))
                    } else {
                        Err("i32")
                    }
                }
                (IntegerBits::Bits32, IntegerSign::Unsigned) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Unsigned32(value)))
                    } else {
                        Err("u32")
                    }
                }
                (IntegerBits::Bits64, IntegerSign::Signed) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Signed64(value)))
                    } else {
                        Err("i64")
                    }
                }
                (IntegerBits::Bits64, IntegerSign::Unsigned) => {
                    if let Ok(value) = value.try_into() {
                        Ok(ir::Value::Literal(Literal::Unsigned64(value)))
                    } else {
                        Err("u64")
                    }
                }
            }
            .map_err(|expected_type| {
                LowerErrorKind::CannotFit {
                    value: value.to_string(),
                    expected_type: expected_type.to_string(),
                }
                .at(expr.source)
            })
        }
        ExprKind::FloatingLiteral(size, value) => Ok(Value::Literal(match size {
            FloatSize::Bits32 => Literal::Float32(*value as f32),
            FloatSize::Bits64 => Literal::Float64(*value),
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
            let callee = resolved_ast
                .functions
                .get(call.function)
                .expect("referenced function to exist");

            let arguments = call
                .arguments
                .iter()
                .map(|argument| {
                    lower_expr(builder, ir_module, &argument.expr, function, resolved_ast)
                })
                .collect::<Result<Box<[_]>, _>>()?;

            let variadic_argument_types = call.arguments[callee.parameters.required.len()..]
                .iter()
                .map(|argument| {
                    lower_type(&ir_module.target, &argument.resolved_type, resolved_ast)
                })
                .collect::<Result<Box<[_]>, _>>()?;

            Ok(builder.push(ir::Instruction::Call(ir::Call {
                function: call.function,
                arguments,
                unpromoted_variadic_argument_types: variadic_argument_types,
            })))
        }
        ExprKind::Variable(variable) => {
            let pointer_to_variable = lower_variable_to_value(variable.key);
            let variable_type =
                lower_type(&ir_module.target, &variable.resolved_type, resolved_ast)?;
            Ok(builder.push(ir::Instruction::Load((pointer_to_variable, variable_type))))
        }
        ExprKind::GlobalVariable(global_variable) => {
            let pointer = builder.push(ir::Instruction::GlobalVariable(global_variable.reference));
            let ir_type = lower_type(
                &ir_module.target,
                &global_variable.resolved_type,
                resolved_ast,
            )?;
            Ok(builder.push(ir::Instruction::Load((pointer, ir_type))))
        }
        ExprKind::DeclareAssign(declare_assign) => {
            let initial_value = lower_expr(
                builder,
                ir_module,
                &declare_assign.value,
                function,
                resolved_ast,
            )?;

            let destination = Value::Reference(ValueReference {
                basicblock_id: 0,
                instruction_id: declare_assign.key.index,
            });

            builder.push(ir::Instruction::Store(ir::Store {
                new_value: initial_value,
                destination: destination.clone(),
            }));

            let ir_type = lower_type(
                &ir_module.target,
                &declare_assign.resolved_type,
                resolved_ast,
            )?;
            Ok(builder.push(ir::Instruction::Load((destination, ir_type))))
        }
        ExprKind::BasicBinaryOperation(operation) => {
            let left = lower_expr(
                builder,
                ir_module,
                &operation.left.expr,
                function,
                resolved_ast,
            )?;
            let right = lower_expr(
                builder,
                ir_module,
                &operation.right.expr,
                function,
                resolved_ast,
            )?;

            lower_basic_binary_operation(
                builder,
                ir_module,
                &operation.operator,
                ir::BinaryOperands::new(left, right),
            )
        }
        ExprKind::ShortCircuitingBinaryOperation(operation) => {
            lower_short_circuiting_binary_operation(
                builder,
                ir_module,
                operation,
                function,
                resolved_ast,
            )
        }
        ExprKind::IntegerCast(cast_from) => {
            integer_cast(builder, ir_module, function, resolved_ast, cast_from)
        }
        ExprKind::IntegerExtend(cast) => {
            integer_extend(builder, ir_module, function, resolved_ast, cast)
        }
        ExprKind::IntegerTruncate(cast) => {
            integer_truncate(builder, ir_module, function, resolved_ast, cast)
        }
        ExprKind::FloatExtend(cast) => {
            let value = lower_expr(builder, ir_module, &cast.value, function, resolved_ast)?;
            let ir_type = lower_type(&ir_module.target, &cast.target_type, resolved_ast)?;
            Ok(builder.push(ir::Instruction::FloatExtend(value, ir_type)))
        }
        ExprKind::Member(member) => {
            let Member {
                subject,
                structure_ref,
                index,
                field_type,
                memory_management,
            } = &**member;

            let subject_pointer =
                lower_destination(builder, ir_module, subject, function, resolved_ast)?;

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

            let ir_type = lower_type(&ir_module.target, field_type, resolved_ast)?;
            Ok(builder.push(ir::Instruction::Load((member, ir_type))))
        }
        ExprKind::ArrayAccess(array_access) => {
            let subject = lower_expr(
                builder,
                ir_module,
                &array_access.subject,
                function,
                resolved_ast,
            )?;
            let index = lower_expr(
                builder,
                ir_module,
                &array_access.index,
                function,
                resolved_ast,
            )?;
            let item_type = lower_type(&ir_module.target, &array_access.item_type, resolved_ast)?;

            let item = builder.push(ir::Instruction::ArrayAccess {
                item_type: item_type.clone(),
                subject_pointer: subject,
                index,
            });

            Ok(builder.push(ir::Instruction::Load((item, item_type))))
        }
        ExprKind::StructureLiteral(structure_literal) => {
            let StructureLiteral {
                structure_type,
                fields,
                memory_management,
            } = &**structure_literal;

            let result_ir_type = lower_type(&ir_module.target, structure_type, resolved_ast)?;
            let mut values = Vec::with_capacity(fields.len());

            // Evaluate field values in the order specified by the struct literal
            for (expr, index) in fields.values() {
                let ir_value = lower_expr(builder, ir_module, expr, function, resolved_ast)?;
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

                    builder.push(ir::Instruction::Store(ir::Store {
                        new_value: ir::Value::Literal(Literal::Unsigned64(1)),
                        destination: at_reference_count,
                    }));

                    let at_value = builder.push(ir::Instruction::Member {
                        subject_pointer: heap_memory.clone(),
                        struct_type: wrapper_struct_type.clone(),
                        index: 1,
                    });

                    builder.push(ir::Instruction::Store(ir::Store {
                        new_value: flat,
                        destination: at_value,
                    }));

                    Ok(heap_memory)
                }
            }
        }
        ExprKind::UnaryOperation(unary_operation) => {
            let inner = lower_expr(
                builder,
                ir_module,
                &unary_operation.inner.expr,
                function,
                resolved_ast,
            )?;

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
                let condition =
                    lower_expr(builder, ir_module, &condition.expr, function, resolved_ast)?;

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
                let value = lower_stmts(builder, ir_module, &block.stmts, function, resolved_ast)?;

                incoming.push(ir::PhiIncoming {
                    basicblock_id: builder.current_block_id(),
                    value,
                });
                builder.continues_to(resume_basicblock_id);

                builder.use_block(false_basicblock_id);
            }

            if let Some(block) = &conditional.otherwise {
                let value = lower_stmts(builder, ir_module, &block.stmts, function, resolved_ast)?;
                incoming.push(ir::PhiIncoming {
                    basicblock_id: builder.current_block_id(),
                    value,
                });
            }

            builder.continues_to(resume_basicblock_id);
            builder.use_block(resume_basicblock_id);

            if conditional.otherwise.is_some() {
                let ir_type =
                    lower_type(&ir_module.target, &conditional.result_type, resolved_ast)?;
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

            let condition = lower_expr(
                builder,
                ir_module,
                &while_loop.condition,
                function,
                resolved_ast,
            )?;

            builder.push(ir::Instruction::ConditionalBreak(
                condition,
                ir::ConditionalBreak {
                    true_basicblock_id,
                    false_basicblock_id,
                },
            ));

            builder.use_block(true_basicblock_id);
            lower_stmts(
                builder,
                ir_module,
                &while_loop.block.stmts,
                function,
                resolved_ast,
            )?;
            builder.continues_to(evaluate_basicblock_id);

            builder.use_block(false_basicblock_id);
            Ok(Value::Literal(Literal::Void))
        }
        ExprKind::EnumMemberLiteral(enum_member_literal) => {
            let enum_definition = resolved_ast
                .enums
                .get(&enum_member_literal.enum_name)
                .expect("referenced enum to exist for enum member literal");

            let member = enum_definition
                .members
                .get(&enum_member_literal.variant_name)
                .ok_or_else(|| {
                    LowerErrorKind::NoSuchEnumMember {
                        enum_name: enum_member_literal.enum_name.clone(),
                        variant_name: enum_member_literal.variant_name.clone(),
                    }
                    .at(enum_member_literal.source)
                })?;

            let ir_type = lower_type(
                &ir_module.target,
                &enum_definition.resolved_type,
                resolved_ast,
            )?;

            let value = &member.value;

            let make_error = |_| {
                LowerErrorKind::CannotFit {
                    value: value.to_string(),
                    expected_type: enum_member_literal.enum_name.clone(),
                }
                .at(enum_definition.source)
            };

            Ok(match ir_type {
                ir::Type::S8 => {
                    ir::Value::Literal(Literal::Signed8(value.try_into().map_err(make_error)?))
                }
                ir::Type::S16 => {
                    ir::Value::Literal(Literal::Signed16(value.try_into().map_err(make_error)?))
                }
                ir::Type::S32 => {
                    ir::Value::Literal(Literal::Signed32(value.try_into().map_err(make_error)?))
                }
                ir::Type::S64 => {
                    ir::Value::Literal(Literal::Signed64(value.try_into().map_err(make_error)?))
                }
                ir::Type::U8 => {
                    ir::Value::Literal(Literal::Unsigned8(value.try_into().map_err(make_error)?))
                }
                ir::Type::U16 => {
                    ir::Value::Literal(Literal::Unsigned16(value.try_into().map_err(make_error)?))
                }
                ir::Type::U32 => {
                    ir::Value::Literal(Literal::Unsigned32(value.try_into().map_err(make_error)?))
                }
                ir::Type::U64 => {
                    ir::Value::Literal(Literal::Unsigned64(value.try_into().map_err(make_error)?))
                }
                _ => {
                    return Err(LowerErrorKind::EnumBackingTypeMustBeInteger {
                        enum_name: enum_member_literal.enum_name.clone(),
                    }
                    .at(enum_definition.source))
                }
            })
        }
        ExprKind::ResolvedNamedExpression(_name, resolved_expr) => {
            lower_expr(builder, ir_module, resolved_expr, function, resolved_ast)
        }
        ExprKind::Zeroed(resolved_type) => {
            let ir_type = lower_type(&ir_module.target, resolved_type, resolved_ast)?;
            Ok(ir::Value::Literal(Literal::Zeroed(ir_type)))
        }
        ExprKind::InterpreterSyscall(syscall, args) => {
            let mut values = Vec::with_capacity(args.len());

            for arg in args {
                values.push(lower_expr(builder, ir_module, arg, function, resolved_ast)?);
            }

            Ok(builder.push(ir::Instruction::InterpreterSyscall(*syscall, values)))
        }
    }
}

pub fn lower_basic_binary_operation(
    builder: &mut Builder,
    ir_module: &ir::Module,
    operator: &resolved::BasicBinaryOperator,
    operands: ir::BinaryOperands,
) -> Result<Value, LowerError> {
    match operator {
        resolved::BasicBinaryOperator::Add(mode) => Ok(builder.push(match mode {
            NumericMode::Integer(_) | NumericMode::LooseIndeterminateSignInteger(_) => {
                ir::Instruction::Add(operands, FloatOrInteger::Integer)
            }
            NumericMode::Float => ir::Instruction::Add(operands, FloatOrInteger::Float),
            NumericMode::CheckOverflow(bits, sign) => ir::Instruction::Checked(
                ir::OverflowOperation {
                    operator: OverflowOperator::Add,
                    bits: *bits,
                    sign: *sign,
                },
                operands,
            ),
        })),
        resolved::BasicBinaryOperator::Subtract(mode) => Ok(builder.push(match mode {
            NumericMode::Integer(_) | NumericMode::LooseIndeterminateSignInteger(_) => {
                ir::Instruction::Subtract(operands, FloatOrInteger::Integer)
            }
            NumericMode::Float => ir::Instruction::Subtract(operands, FloatOrInteger::Float),
            NumericMode::CheckOverflow(bits, sign) => ir::Instruction::Checked(
                ir::OverflowOperation {
                    operator: OverflowOperator::Subtract,
                    bits: *bits,
                    sign: *sign,
                },
                operands,
            ),
        })),
        resolved::BasicBinaryOperator::Multiply(mode) => Ok(builder.push(match mode {
            NumericMode::Integer(_) | NumericMode::LooseIndeterminateSignInteger(_) => {
                ir::Instruction::Multiply(operands, FloatOrInteger::Integer)
            }
            NumericMode::Float => ir::Instruction::Multiply(operands, FloatOrInteger::Float),
            NumericMode::CheckOverflow(bits, sign) => ir::Instruction::Checked(
                ir::OverflowOperation {
                    operator: OverflowOperator::Multiply,
                    bits: *bits,
                    sign: *sign,
                },
                operands,
            ),
        })),
        resolved::BasicBinaryOperator::Divide(mode) => Ok(builder.push(ir::Instruction::Divide(
            operands,
            mode.or_default_for(ir_module.target),
        ))),
        resolved::BasicBinaryOperator::Modulus(mode) => Ok(builder.push(ir::Instruction::Modulus(
            operands,
            mode.or_default_for(ir_module.target),
        ))),
        resolved::BasicBinaryOperator::Equals(mode) => {
            Ok(builder.push(ir::Instruction::Equals(operands, *mode)))
        }
        resolved::BasicBinaryOperator::NotEquals(mode) => {
            Ok(builder.push(ir::Instruction::NotEquals(operands, *mode)))
        }
        resolved::BasicBinaryOperator::LessThan(mode) => Ok(builder.push(
            ir::Instruction::LessThan(operands, mode.or_default_for(ir_module.target)),
        )),
        resolved::BasicBinaryOperator::LessThanEq(mode) => Ok(builder.push(
            ir::Instruction::LessThanEq(operands, mode.or_default_for(ir_module.target)),
        )),
        resolved::BasicBinaryOperator::GreaterThan(mode) => Ok(builder.push(
            ir::Instruction::GreaterThan(operands, mode.or_default_for(ir_module.target)),
        )),
        resolved::BasicBinaryOperator::GreaterThanEq(mode) => Ok(builder.push(
            ir::Instruction::GreaterThanEq(operands, mode.or_default_for(ir_module.target)),
        )),
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
        resolved::BasicBinaryOperator::ArithmeticRightShift(sign_or_indeterminate) => {
            let sign = match sign_or_indeterminate {
                SignOrIndeterminate::Sign(sign) => *sign,
                SignOrIndeterminate::Indeterminate(c_integer) => {
                    ir_module.target.default_c_integer_sign(*c_integer)
                }
            };

            Ok(builder.push(match sign {
                IntegerSign::Signed => ir::Instruction::ArithmeticRightShift(operands),
                IntegerSign::Unsigned => ir::Instruction::LogicalRightShift(operands),
            }))
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
    resolved_ast: &resolved::Ast,
) -> Result<Value, LowerError> {
    let short_circuit =
        lower_pre_short_circuit(builder, ir_module, operation, function, resolved_ast)?;
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
    resolved_ast: &resolved::Ast,
) -> Result<BinaryShortCircuit, LowerError> {
    let left = lower_expr(
        builder,
        ir_module,
        &operation.left.expr,
        function,
        resolved_ast,
    )?;

    let left_done_block_id = builder.current_block_id();
    let evaluate_right_block_id = builder.new_block();
    builder.use_block(evaluate_right_block_id);

    let right = lower_expr(
        builder,
        ir_module,
        &operation.right.expr,
        function,
        resolved_ast,
    )?;
    let right_done_block_id = builder.current_block_id();

    Ok(BinaryShortCircuit {
        left,
        right,
        left_done_block_id,
        right_done_block_id,
        evaluate_right_block_id,
    })
}

pub fn lower_c_integer(target: &Target, integer: CInteger, sign: Option<IntegerSign>) -> ir::Type {
    let sign = sign.unwrap_or_else(|| target.default_c_integer_sign(integer));

    match (integer, sign) {
        (CInteger::Char, IntegerSign::Signed) => ir::Type::S8,
        (CInteger::Char, IntegerSign::Unsigned) => ir::Type::U8,
        (CInteger::Short, IntegerSign::Signed) => ir::Type::S16,
        (CInteger::Short, IntegerSign::Unsigned) => ir::Type::U16,
        (CInteger::Int, IntegerSign::Signed) => ir::Type::S32,
        (CInteger::Int, IntegerSign::Unsigned) => ir::Type::U32,
        (CInteger::Long, IntegerSign::Signed) => {
            if target.os().is_windows() {
                ir::Type::S32
            } else {
                ir::Type::S64
            }
        }
        (CInteger::Long, IntegerSign::Unsigned) => {
            if target.os().is_windows() {
                ir::Type::U32
            } else {
                ir::Type::U64
            }
        }
        (CInteger::LongLong, IntegerSign::Signed) => ir::Type::S64,
        (CInteger::LongLong, IntegerSign::Unsigned) => ir::Type::U64,
    }
}
