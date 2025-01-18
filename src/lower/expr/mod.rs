mod call;
mod short_circuit;

use super::{
    builder::Builder,
    cast::{integer_cast, integer_extend, integer_truncate},
    datatype::lower_type,
    error::{LowerError, LowerErrorKind},
    stmts::lower_stmts,
};
use crate::{
    asg::{
        self, Asg, Destination, DestinationKind, Expr, ExprKind, FloatOrInteger, Member,
        NumericMode, SignOrIndeterminate, StructLiteral, UnaryMathOperation, VariableStorageKey,
    },
    ast::{FloatSize, IntegerBits, IntegerRigidity},
    ir::{self, IntegerSign, Literal, OverflowOperator, Value, ValueReference},
    lower::structure::mono,
    resolve::PolyCatalog,
};
use call::{lower_expr_call, lower_expr_poly_call};
use short_circuit::lower_short_circuiting_binary_operation;

pub fn lower_expr(
    builder: &mut Builder,
    ir_module: &ir::Module,
    expr: &Expr,
    function: &asg::Func,
    asg: &Asg,
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
        ExprKind::Call(call) => lower_expr_call(builder, ir_module, expr, function, asg, call),
        ExprKind::Variable(variable) => {
            let pointer_to_variable = lower_variable_to_value(variable.key);
            let variable_type = lower_type(ir_module, &builder.unpoly(&variable.ty)?, asg)?;
            Ok(builder.push(ir::Instr::Load((pointer_to_variable, variable_type))))
        }
        ExprKind::GlobalVariable(global_variable) => {
            let pointer = builder.push(ir::Instr::GlobalVariable(global_variable.reference));
            let ir_type = lower_type(ir_module, &builder.unpoly(&global_variable.ty)?, asg)?;
            Ok(builder.push(ir::Instr::Load((pointer, ir_type))))
        }
        ExprKind::DeclareAssign(declare_assign) => {
            let initial_value =
                lower_expr(builder, ir_module, &declare_assign.value, function, asg)?;

            let destination = Value::Reference(ValueReference {
                basicblock_id: 0,
                instruction_id: declare_assign.key.index,
            });

            builder.push(ir::Instr::Store(ir::Store {
                new_value: initial_value,
                destination: destination.clone(),
            }));

            let ir_type = lower_type(ir_module, &builder.unpoly(&declare_assign.ty)?, asg)?;
            Ok(builder.push(ir::Instr::Load((destination, ir_type))))
        }
        ExprKind::BasicBinaryOperation(operation) => {
            let left = lower_expr(builder, ir_module, &operation.left.expr, function, asg)?;
            let right = lower_expr(builder, ir_module, &operation.right.expr, function, asg)?;

            lower_basic_binary_operation(
                builder,
                ir_module,
                &operation.operator,
                ir::BinaryOperands::new(left, right),
            )
        }
        ExprKind::ShortCircuitingBinaryOperation(operation) => {
            lower_short_circuiting_binary_operation(builder, ir_module, operation, function, asg)
        }
        ExprKind::IntegerCast(cast_from) => {
            integer_cast(builder, ir_module, function, asg, cast_from)
        }
        ExprKind::IntegerExtend(cast_from) => {
            integer_extend(builder, ir_module, function, asg, cast_from)
        }
        ExprKind::IntegerTruncate(cast) => {
            integer_truncate(builder, ir_module, function, asg, cast)
        }
        ExprKind::FloatExtend(cast) => {
            let value = lower_expr(builder, ir_module, &cast.value, function, asg)?;
            let ir_type = lower_type(ir_module, &builder.unpoly(&cast.target_type)?, asg)?;
            Ok(builder.push(ir::Instr::FloatExtend(value, ir_type)))
        }
        ExprKind::FloatToInteger(cast) => {
            let value = lower_expr(builder, ir_module, &cast.value, function, asg)?;
            let ir_type = lower_type(ir_module, &builder.unpoly(&cast.target_type)?, asg)?;
            let sign = if ir_type
                .is_signed()
                .expect("must know signness in order to cast float to integer")
            {
                IntegerSign::Signed
            } else {
                IntegerSign::Unsigned
            };

            Ok(builder.push(ir::Instr::FloatToInteger(value, ir_type, sign)))
        }
        ExprKind::IntegerToFloat(cast_from) => {
            let cast = &cast_from.cast;
            let from_sign = cast_from
                .from_type
                .kind
                .sign(Some(&ir_module.target))
                .expect("integer to float must know sign");

            let value = lower_expr(builder, ir_module, &cast.value, function, asg)?;
            let ir_type = lower_type(ir_module, &builder.unpoly(&cast.target_type)?, asg)?;
            Ok(builder.push(ir::Instr::IntegerToFloat(value, ir_type, from_sign)))
        }
        ExprKind::Member(member) => {
            let Member {
                subject,
                struct_ref,
                index,
                field_type,
            } = &**member;

            let asg::TypeKind::Structure(_name, _struct_ref, arguments) = &subject.ty.kind else {
                todo!("member operator only supports structure types for now");
            };

            let subject_pointer = lower_destination(builder, ir_module, subject, function, asg)?;

            let structure = asg
                .structs
                .get(*struct_ref)
                .expect("referenced structure to exist");

            assert!(structure.params.len() == arguments.len());
            let mut catalog = PolyCatalog::new();
            for (name, argument) in structure.params.names().zip(arguments.iter()) {
                catalog
                    .put_type(name, &builder.unpoly(argument)?.0)
                    .expect("unique type parameter names");
            }
            let poly_recipe = catalog.bake();

            let struct_ref =
                ir_module
                    .structs
                    .translate(*struct_ref, poly_recipe, |poly_recipe| {
                        mono(ir_module, asg, *struct_ref, poly_recipe)
                    })?;

            // Access member of structure
            let member = builder.push(ir::Instr::Member {
                subject_pointer,
                struct_type: ir::Type::Struct(struct_ref),
                index: *index,
            });

            let ir_type = lower_type(ir_module, &builder.unpoly(field_type)?, asg)?;
            Ok(builder.push(ir::Instr::Load((member, ir_type))))
        }
        ExprKind::ArrayAccess(array_access) => {
            let subject = lower_expr(builder, ir_module, &array_access.subject, function, asg)?;
            let index = lower_expr(builder, ir_module, &array_access.index, function, asg)?;
            let item_type = lower_type(ir_module, &builder.unpoly(&array_access.item_type)?, asg)?;

            let item = builder.push(ir::Instr::ArrayAccess {
                item_type: item_type.clone(),
                subject_pointer: subject,
                index,
            });

            Ok(builder.push(ir::Instr::Load((item, item_type))))
        }
        ExprKind::StructLiteral(struct_literal) => {
            let StructLiteral {
                struct_type,
                fields,
            } = &**struct_literal;

            let result_ir_type = lower_type(ir_module, &builder.unpoly(struct_type)?, asg)?;
            let mut values = Vec::with_capacity(fields.len());

            // Evaluate field values in the order specified by the struct literal
            for (_, expr, index) in fields.iter() {
                let ir_value = lower_expr(builder, ir_module, expr, function, asg)?;
                values.push((index, ir_value));
            }

            // Sort resulting values by index
            values.sort_by(|(a, _), (b, _)| a.cmp(b));

            // Drop the index part of the values
            let values = values.drain(..).map(|(_, value)| value).collect();

            Ok(builder.push(ir::Instr::StructLiteral(result_ir_type, values)))
        }
        ExprKind::UnaryMathOperation(operation) => {
            let UnaryMathOperation { operator, inner } = &**operation;
            let value = lower_expr(builder, ir_module, &inner.expr, function, asg)?;

            let float_or_int = inner
                .ty
                .kind
                .is_float_like()
                .then_some(FloatOrInteger::Float)
                .unwrap_or(FloatOrInteger::Integer);

            let instruction = match operator {
                asg::UnaryMathOperator::Not => ir::Instr::IsZero(value, float_or_int),
                asg::UnaryMathOperator::BitComplement => ir::Instr::BitComplement(value),
                asg::UnaryMathOperator::Negate => ir::Instr::Negate(value, float_or_int),
                asg::UnaryMathOperator::IsNonZero => ir::Instr::IsNonZero(value, float_or_int),
            };

            Ok(builder.push(instruction))
        }
        ExprKind::AddressOf(destination) => Ok(lower_destination(
            builder,
            ir_module,
            destination,
            function,
            asg,
        )?),
        ExprKind::Dereference(subject) => {
            let ir_type = lower_type(ir_module, &builder.unpoly(&subject.ty)?, asg)?;

            let ir::Type::Ptr(ir_type) = ir_type else {
                panic!("Cannot lower dereference of non-pointer");
            };

            let value = lower_expr(builder, ir_module, &subject.expr, function, asg)?;
            Ok(builder.push(ir::Instr::Load((value, *ir_type))))
        }
        ExprKind::Conditional(conditional) => {
            let resume_basicblock_id = builder.new_block();

            let mut incoming = vec![];

            for asg::Branch { condition, block } in conditional.branches.iter() {
                let condition = lower_expr(builder, ir_module, &condition.expr, function, asg)?;

                let true_basicblock_id = builder.new_block();
                let false_basicblock_id = builder.new_block();

                builder.push(ir::Instr::ConditionalBreak(
                    condition,
                    ir::ConditionalBreak {
                        true_basicblock_id,
                        false_basicblock_id,
                    },
                ));

                builder.use_block(true_basicblock_id);
                let value = lower_stmts(builder, ir_module, &block.stmts, function, asg)?;

                incoming.push(ir::PhiIncoming {
                    basicblock_id: builder.current_block_id(),
                    value,
                });
                builder.continues_to(resume_basicblock_id);

                builder.use_block(false_basicblock_id);
            }

            if let Some(block) = &conditional.otherwise {
                let value = lower_stmts(builder, ir_module, &block.stmts, function, asg)?;
                incoming.push(ir::PhiIncoming {
                    basicblock_id: builder.current_block_id(),
                    value,
                });
            }

            builder.continues_to(resume_basicblock_id);
            builder.use_block(resume_basicblock_id);

            if conditional.otherwise.is_some() {
                if let Some(result_type) = &conditional.result_type {
                    let ir_type = lower_type(ir_module, &builder.unpoly(result_type)?, asg)?;
                    Ok(builder.push(ir::Instr::Phi(ir::Phi { ir_type, incoming })))
                } else {
                    Ok(Value::Literal(Literal::Void))
                }
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

            let condition = lower_expr(builder, ir_module, &while_loop.condition, function, asg)?;

            builder.push(ir::Instr::ConditionalBreak(
                condition,
                ir::ConditionalBreak {
                    true_basicblock_id,
                    false_basicblock_id,
                },
            ));

            builder.use_block(true_basicblock_id);
            lower_stmts(builder, ir_module, &while_loop.block.stmts, function, asg)?;
            builder.continues_to(evaluate_basicblock_id);

            builder.use_block(false_basicblock_id);
            Ok(Value::Literal(Literal::Void))
        }
        ExprKind::EnumMemberLiteral(enum_member_literal) => {
            let enum_definition = asg
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

            let ir_type = lower_type(ir_module, &builder.unpoly(&enum_definition.ty)?, asg)?;

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
            lower_expr(builder, ir_module, resolved_expr, function, asg)
        }
        ExprKind::Zeroed(ty) => {
            let ir_type = lower_type(ir_module, &builder.unpoly(ty)?, asg)?;
            Ok(ir::Value::Literal(Literal::Zeroed(ir_type)))
        }
        ExprKind::InterpreterSyscall(syscall, args) => {
            let mut values = Vec::with_capacity(args.len());

            for arg in args {
                values.push(lower_expr(builder, ir_module, arg, function, asg)?);
            }

            Ok(builder.push(ir::Instr::InterpreterSyscall(*syscall, values)))
        }
        ExprKind::PolyCall(poly_call) => {
            lower_expr_poly_call(builder, ir_module, expr, function, asg, poly_call)
        }
    }
}

pub fn lower_destination(
    builder: &mut Builder,
    ir_module: &ir::Module,
    destination: &Destination,
    function: &asg::Func,
    asg: &Asg,
) -> Result<ir::Value, LowerError> {
    match &destination.kind {
        DestinationKind::Variable(variable) => Ok(lower_variable_to_value(variable.key)),
        DestinationKind::GlobalVariable(global_variable) => {
            let pointer = builder.push(ir::Instr::GlobalVariable(global_variable.reference));
            Ok(pointer)
        }
        DestinationKind::Member {
            subject,
            struct_ref,
            index,
            ..
        } => {
            let subject_pointer = lower_destination(builder, ir_module, subject, function, asg)?;

            let asg::TypeKind::Structure(_name, _struct_ref, arguments) = &subject.ty.kind else {
                todo!("member operator only supports structure types for now");
            };

            let structure = asg
                .structs
                .get(*struct_ref)
                .expect("referenced structure to exist");

            assert!(structure.params.len() == arguments.len());
            let mut catalog = PolyCatalog::new();
            for (name, argument) in structure.params.names().zip(arguments.iter()) {
                catalog
                    .put_type(name, &builder.unpoly(argument)?.0)
                    .expect("unique type parameter names");
            }
            let poly_recipe = catalog.bake();

            let struct_ref =
                ir_module
                    .structs
                    .translate(*struct_ref, poly_recipe, |poly_recipe| {
                        mono(ir_module, asg, *struct_ref, poly_recipe)
                    })?;

            Ok(builder.push(ir::Instr::Member {
                subject_pointer,
                struct_type: ir::Type::Struct(struct_ref),
                index: *index,
            }))
        }
        DestinationKind::ArrayAccess(array_access) => {
            let subject_pointer =
                lower_expr(builder, ir_module, &array_access.subject, function, asg)?;
            let index = lower_expr(builder, ir_module, &array_access.index, function, asg)?;
            let item_type = lower_type(ir_module, &builder.unpoly(&array_access.item_type)?, asg)?;

            Ok(builder.push(ir::Instr::ArrayAccess {
                item_type,
                subject_pointer,
                index,
            }))
        }
        DestinationKind::Dereference(lvalue) => {
            Ok(lower_expr(builder, ir_module, lvalue, function, asg)?)
        }
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
            ir::Instr::Add(operands, FloatOrInteger::Integer)
        }
        NumericMode::Float => ir::Instr::Add(operands, FloatOrInteger::Float),
        NumericMode::CheckOverflow(bits, sign) => ir::Instr::Checked(
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
    operator: &asg::BasicBinaryOperator,
    operands: ir::BinaryOperands,
) -> Result<Value, LowerError> {
    match operator {
        asg::BasicBinaryOperator::PrimitiveAdd(ty) => {
            let ty = builder.unpoly(ty)?;
            let numeric_mode = NumericMode::try_new(&ty.0).expect("PrimitiveAdd to be addable");
            lower_add(builder, &numeric_mode, operands)
        }
        asg::BasicBinaryOperator::Add(mode) => lower_add(builder, mode, operands),
        asg::BasicBinaryOperator::Subtract(mode) => Ok(builder.push(match mode {
            NumericMode::Integer(_) | NumericMode::LooseIndeterminateSignInteger(_) => {
                ir::Instr::Subtract(operands, FloatOrInteger::Integer)
            }
            NumericMode::Float => ir::Instr::Subtract(operands, FloatOrInteger::Float),
            NumericMode::CheckOverflow(bits, sign) => ir::Instr::Checked(
                ir::OverflowOperation {
                    operator: OverflowOperator::Subtract,
                    bits: *bits,
                    sign: *sign,
                },
                operands,
            ),
        })),
        asg::BasicBinaryOperator::Multiply(mode) => Ok(builder.push(match mode {
            NumericMode::Integer(_) | NumericMode::LooseIndeterminateSignInteger(_) => {
                ir::Instr::Multiply(operands, FloatOrInteger::Integer)
            }
            NumericMode::Float => ir::Instr::Multiply(operands, FloatOrInteger::Float),
            NumericMode::CheckOverflow(bits, sign) => ir::Instr::Checked(
                ir::OverflowOperation {
                    operator: OverflowOperator::Multiply,
                    bits: *bits,
                    sign: *sign,
                },
                operands,
            ),
        })),
        asg::BasicBinaryOperator::Divide(mode) => Ok(builder.push(ir::Instr::Divide(
            operands,
            mode.or_default_for(&ir_module.target),
        ))),
        asg::BasicBinaryOperator::Modulus(mode) => Ok(builder.push(ir::Instr::Modulus(
            operands,
            mode.or_default_for(&ir_module.target),
        ))),
        asg::BasicBinaryOperator::Equals(mode) => {
            Ok(builder.push(ir::Instr::Equals(operands, *mode)))
        }
        asg::BasicBinaryOperator::NotEquals(mode) => {
            Ok(builder.push(ir::Instr::NotEquals(operands, *mode)))
        }
        asg::BasicBinaryOperator::LessThan(mode) => Ok(builder.push(ir::Instr::LessThan(
            operands,
            mode.or_default_for(&ir_module.target),
        ))),
        asg::BasicBinaryOperator::LessThanEq(mode) => Ok(builder.push(ir::Instr::LessThanEq(
            operands,
            mode.or_default_for(&ir_module.target),
        ))),
        asg::BasicBinaryOperator::GreaterThan(mode) => Ok(builder.push(ir::Instr::GreaterThan(
            operands,
            mode.or_default_for(&ir_module.target),
        ))),
        asg::BasicBinaryOperator::GreaterThanEq(mode) => Ok(builder.push(
            ir::Instr::GreaterThanEq(operands, mode.or_default_for(&ir_module.target)),
        )),
        asg::BasicBinaryOperator::BitwiseAnd => Ok(builder.push(ir::Instr::BitwiseAnd(operands))),
        asg::BasicBinaryOperator::BitwiseOr => Ok(builder.push(ir::Instr::BitwiseOr(operands))),
        asg::BasicBinaryOperator::BitwiseXor => Ok(builder.push(ir::Instr::BitwiseXor(operands))),
        asg::BasicBinaryOperator::LogicalLeftShift | asg::BasicBinaryOperator::LeftShift => {
            Ok(builder.push(ir::Instr::LeftShift(operands)))
        }
        asg::BasicBinaryOperator::ArithmeticRightShift(sign_or_indeterminate) => {
            let sign = match sign_or_indeterminate {
                SignOrIndeterminate::Sign(sign) => *sign,
                SignOrIndeterminate::Indeterminate(c_integer) => {
                    ir_module.target.default_c_integer_sign(*c_integer)
                }
            };

            Ok(builder.push(match sign {
                IntegerSign::Signed => ir::Instr::ArithmeticRightShift(operands),
                IntegerSign::Unsigned => ir::Instr::LogicalRightShift(operands),
            }))
        }
        asg::BasicBinaryOperator::LogicalRightShift => {
            Ok(builder.push(ir::Instr::LogicalRightShift(operands)))
        }
    }
}
