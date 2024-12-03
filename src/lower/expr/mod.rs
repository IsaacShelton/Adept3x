mod short_circuit;

use super::{
    builder::Builder,
    cast::{integer_cast, integer_extend, integer_truncate},
    datatype::lower_type,
    error::{LowerError, LowerErrorKind},
    function::lower_function_head,
    stmts::lower_stmts,
};
use crate::{
    ast::{FloatSize, IntegerBits, IntegerRigidity},
    ir::{self, IntegerSign, Literal, OverflowOperator, Value, ValueReference},
    resolved::{
        self, Destination, DestinationKind, Expr, ExprKind, FloatOrInteger, Member, NumericMode,
        SignOrIndeterminate, StructLiteral, UnaryMathOperation, VariableStorageKey,
    },
};
use short_circuit::lower_short_circuiting_binary_operation;

pub fn lower_expr(
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

            let (bits, sign) = match &integer.rigidity {
                IntegerRigidity::Fixed(bits, sign) => (*bits, *sign),
                IntegerRigidity::Loose(c_integer, sign) => {
                    let bits = IntegerBits::try_from(c_integer.bytes(&ir_module.target))
                        .expect("supported integer size");

                    let sign =
                        sign.unwrap_or_else(|| ir_module.target.default_c_integer_sign(*c_integer));

                    (bits, sign)
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
            FloatSize::Bits32 => {
                Literal::Float32(value.map(|x| x.as_f32().into_inner()).unwrap_or(f32::NAN))
            }
            FloatSize::Bits64 => {
                Literal::Float64(value.map(|x| x.into_inner()).unwrap_or(f64::NAN))
            }
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
                .get(call.callee.function)
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
                    lower_type(
                        &ir_module.target,
                        &builder.unpoly(&argument.resolved_type)?,
                        resolved_ast,
                    )
                })
                .collect::<Result<Box<[_]>, _>>()?;

            let function =
                ir_module
                    .functions
                    .translate(call.callee.function, &call.callee.recipe, || {
                        lower_function_head(
                            ir_module,
                            call.callee.function,
                            &call.callee.recipe,
                            resolved_ast,
                        )
                    })?;

            Ok(builder.push(ir::Instruction::Call(ir::Call {
                function,
                arguments,
                unpromoted_variadic_argument_types: variadic_argument_types,
            })))
        }
        ExprKind::Variable(variable) => {
            let pointer_to_variable = lower_variable_to_value(variable.key);
            let variable_type = lower_type(
                &ir_module.target,
                &builder.unpoly(&variable.resolved_type)?,
                resolved_ast,
            )?;
            Ok(builder.push(ir::Instruction::Load((pointer_to_variable, variable_type))))
        }
        ExprKind::GlobalVariable(global_variable) => {
            let pointer = builder.push(ir::Instruction::GlobalVariable(global_variable.reference));
            let ir_type = lower_type(
                &ir_module.target,
                &builder.unpoly(&global_variable.resolved_type)?,
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
                &builder.unpoly(&declare_assign.resolved_type)?,
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
        ExprKind::IntegerExtend(cast_from) => {
            integer_extend(builder, ir_module, function, resolved_ast, cast_from)
        }
        ExprKind::IntegerTruncate(cast) => {
            integer_truncate(builder, ir_module, function, resolved_ast, cast)
        }
        ExprKind::FloatExtend(cast) => {
            let value = lower_expr(builder, ir_module, &cast.value, function, resolved_ast)?;
            let ir_type = lower_type(
                &ir_module.target,
                &builder.unpoly(&cast.target_type)?,
                resolved_ast,
            )?;
            Ok(builder.push(ir::Instruction::FloatExtend(value, ir_type)))
        }
        ExprKind::FloatToInteger(cast) => {
            let value = lower_expr(builder, ir_module, &cast.value, function, resolved_ast)?;
            let ir_type = lower_type(
                &ir_module.target,
                &builder.unpoly(&cast.target_type)?,
                resolved_ast,
            )?;
            let sign = if ir_type
                .is_signed()
                .expect("must know signness in order to cast float to integer")
            {
                IntegerSign::Signed
            } else {
                IntegerSign::Unsigned
            };

            Ok(builder.push(ir::Instruction::FloatToInteger(value, ir_type, sign)))
        }
        ExprKind::IntegerToFloat(cast_from) => {
            let cast = &cast_from.cast;
            let from_sign = cast_from
                .from_type
                .kind
                .sign(Some(ir_module.target))
                .expect("integer to float must know sign");

            let value = lower_expr(builder, ir_module, &cast.value, function, resolved_ast)?;
            let ir_type = lower_type(
                &ir_module.target,
                &builder.unpoly(&cast.target_type)?,
                resolved_ast,
            )?;
            Ok(builder.push(ir::Instruction::IntegerToFloat(value, ir_type, from_sign)))
        }
        ExprKind::Member(member) => {
            let Member {
                subject,
                structure_ref: resolved_structure_ref,
                poly_recipe,
                index,
                field_type,
            } = &**member;

            let subject_pointer =
                lower_destination(builder, ir_module, subject, function, resolved_ast)?;

            let structure_ref =
                ir_module
                    .structures
                    .translate(*resolved_structure_ref, poly_recipe, || {
                        todo!("monomorphize structure for lowering member expression");

                        #[allow(unreachable_code)]
                        Err(LowerErrorKind::CannotFit {
                            value: "oops".into(),
                            expected_type:
                                "lower_expr translate resolved structure reference is unimplemented"
                                    .into(),
                        }
                        .at(expr.source))
                    })?;

            // Access member of structure
            let member = builder.push(ir::Instruction::Member {
                subject_pointer,
                struct_type: ir::Type::Structure(structure_ref),
                index: *index,
            });

            let ir_type = lower_type(
                &ir_module.target,
                &builder.unpoly(field_type)?,
                resolved_ast,
            )?;
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
            let item_type = lower_type(
                &ir_module.target,
                &builder.unpoly(&array_access.item_type)?,
                resolved_ast,
            )?;

            let item = builder.push(ir::Instruction::ArrayAccess {
                item_type: item_type.clone(),
                subject_pointer: subject,
                index,
            });

            Ok(builder.push(ir::Instruction::Load((item, item_type))))
        }
        ExprKind::StructLiteral(structure_literal) => {
            let StructLiteral {
                structure_type,
                fields,
            } = &**structure_literal;

            let result_ir_type = lower_type(
                &ir_module.target,
                &builder.unpoly(structure_type)?,
                resolved_ast,
            )?;
            let mut values = Vec::with_capacity(fields.len());

            // Evaluate field values in the order specified by the struct literal
            for (_, expr, index) in fields.iter() {
                let ir_value = lower_expr(builder, ir_module, expr, function, resolved_ast)?;
                values.push((index, ir_value));
            }

            // Sort resulting values by index
            values.sort_by(|(a, _), (b, _)| a.cmp(b));

            // Drop the index part of the values
            let values = values.drain(..).map(|(_, value)| value).collect();

            Ok(builder.push(ir::Instruction::StructLiteral(result_ir_type, values)))
        }
        ExprKind::UnaryMathOperation(operation) => {
            let UnaryMathOperation { operator, inner } = &**operation;
            let value = lower_expr(builder, ir_module, &inner.expr, function, resolved_ast)?;

            let float_or_int = inner
                .resolved_type
                .kind
                .is_float_like()
                .then_some(FloatOrInteger::Float)
                .unwrap_or(FloatOrInteger::Integer);

            let instruction = match operator {
                resolved::UnaryMathOperator::Not => ir::Instruction::IsZero(value, float_or_int),
                resolved::UnaryMathOperator::BitComplement => ir::Instruction::BitComplement(value),
                resolved::UnaryMathOperator::Negate => ir::Instruction::Negate(value, float_or_int),
                resolved::UnaryMathOperator::IsNonZero => {
                    ir::Instruction::IsNonZero(value, float_or_int)
                }
            };

            Ok(builder.push(instruction))
        }
        ExprKind::AddressOf(destination) => Ok(lower_destination(
            builder,
            ir_module,
            destination,
            function,
            resolved_ast,
        )?),
        ExprKind::Dereference(subject) => {
            let ir_type = lower_type(
                ir_module.target,
                &builder.unpoly(&subject.resolved_type)?,
                resolved_ast,
            )?;
            let value = lower_expr(builder, ir_module, &subject.expr, function, resolved_ast)?;
            Ok(builder.push(ir::Instruction::Load((value, ir_type))))
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
                let ir_type = lower_type(
                    &ir_module.target,
                    &builder.unpoly(&conditional.result_type)?,
                    resolved_ast,
                )?;
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
                .get(enum_member_literal.enum_ref)
                .expect("referenced enum to exist for enum member literal");

            let member = enum_definition
                .members
                .get(&enum_member_literal.variant_name)
                .ok_or_else(|| {
                    LowerErrorKind::NoSuchEnumMember {
                        enum_name: enum_member_literal.human_name.to_string(),
                        variant_name: enum_member_literal.variant_name.clone(),
                    }
                    .at(enum_member_literal.source)
                })?;

            let ir_type = lower_type(
                &ir_module.target,
                &builder.unpoly(&enum_definition.resolved_type)?,
                resolved_ast,
            )?;

            let value = &member.value;

            let make_error = |_| {
                LowerErrorKind::CannotFit {
                    value: value.to_string(),
                    expected_type: enum_member_literal.human_name.to_string(),
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
                        enum_name: enum_member_literal.human_name.to_string(),
                    }
                    .at(enum_definition.source))
                }
            })
        }
        ExprKind::ResolvedNamedExpression(resolved_expr) => {
            lower_expr(builder, ir_module, resolved_expr, function, resolved_ast)
        }
        ExprKind::Zeroed(resolved_type) => {
            let ir_type = lower_type(
                &ir_module.target,
                &builder.unpoly(resolved_type)?,
                resolved_ast,
            )?;
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

pub fn lower_destination(
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
            structure_ref: resolved_structure_ref,
            poly_recipe,
            index,
            ..
        } => {
            let subject_pointer =
                lower_destination(builder, ir_module, subject, function, resolved_ast)?;

            let structure_ref =
                ir_module
                    .structures
                    .translate(*resolved_structure_ref, poly_recipe, || {
                        todo!("monomorphize structure for lowering member expression 2");

                        #[allow(unreachable_code)]
                        Err(LowerErrorKind::CannotFit {
                            value: "oops".into(),
                            expected_type:
                                "lower_destination translate resolved structure reference is unimplemented"
                                    .into(),
                        }
                        .at(destination.source))
                    })?;

            Ok(builder.push(ir::Instruction::Member {
                subject_pointer,
                struct_type: ir::Type::Structure(structure_ref),
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
            let item_type = lower_type(
                &ir_module.target,
                &builder.unpoly(&array_access.item_type)?,
                resolved_ast,
            )?;

            Ok(builder.push(ir::Instruction::ArrayAccess {
                item_type,
                subject_pointer,
                index,
            }))
        }
        DestinationKind::Dereference(lvalue) => Ok(lower_expr(
            builder,
            ir_module,
            lvalue,
            function,
            resolved_ast,
        )?),
    }
}

fn lower_variable_to_value(key: VariableStorageKey) -> Value {
    Value::Reference(ValueReference {
        basicblock_id: 0,
        instruction_id: key.index,
    })
}

fn lower_add(
    builder: &mut Builder,
    mode: &NumericMode,
    operands: ir::BinaryOperands,
) -> Result<Value, LowerError> {
    Ok(builder.push(match mode {
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
    }))
}

pub fn lower_basic_binary_operation(
    builder: &mut Builder,
    ir_module: &ir::Module,
    operator: &resolved::BasicBinaryOperator,
    operands: ir::BinaryOperands,
) -> Result<Value, LowerError> {
    match operator {
        resolved::BasicBinaryOperator::PrimitiveAdd(resolved_type) => {
            let ty = builder.unpoly(resolved_type)?;
            let numeric_mode = NumericMode::try_new(&ty.0).expect("PrimitiveAdd to be addable");
            lower_add(builder, &numeric_mode, operands)
        }
        resolved::BasicBinaryOperator::Add(mode) => lower_add(builder, mode, operands),
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
