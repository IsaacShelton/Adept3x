mod call;
mod short_circuit;

use super::{
    cast::{integer_cast, integer_extend, integer_truncate},
    error::{LowerError, LowerErrorKind},
    func_builder::FuncBuilder,
    stmts::{lower_stmts, lower_stmts_with_break_and_continue},
};
use crate::{ModBuilder, structure::monomorphize_struct};
use asg::{Destination, DestinationKind, ExprKind, PolyCatalog, VariableStorageKey};
use call::{lower_expr_call, lower_expr_poly_call};
use data_units::BitUnits;
use ir::{Literal, OverflowOperator, Value, ValueReference};
use num::{BigInt, FromPrimitive};
use primitives::{
    FloatOrInteger, FloatSize, IntegerBits, IntegerRigidity, IntegerSign, NumericMode,
    SignOrIndeterminate,
};
use short_circuit::lower_short_circuiting_binary_operation;
use target_layout::TargetLayout;

pub fn lower_expr(builder: &mut FuncBuilder, expr: &asg::Expr) -> Result<ir::Value, LowerError> {
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
                    let bits = IntegerBits::try_from(builder.target().c_integer_bytes(*c_integer))
                        .expect("supported integer size");

                    let sign =
                        sign.unwrap_or_else(|| builder.target().default_c_integer_sign(*c_integer));

                    (bits, sign)
                }
                IntegerRigidity::Size(sign) => {
                    let bits = IntegerBits::try_from(builder.target().size_layout().width)
                        .expect("supported integer size");
                    (bits, *sign)
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
        ExprKind::FloatingLiteral(size, value) => Ok(ir::Value::Literal(match size {
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
        ExprKind::Null => Ok(ir::Value::Literal(Literal::Zeroed(ir::Type::Ptr(
            Box::new(ir::Type::Void),
        )))),
        ExprKind::Call(call) => lower_expr_call(builder, expr, call),
        ExprKind::PolyCall(poly_call) => lower_expr_poly_call(builder, expr, poly_call),
        ExprKind::Variable(variable) => {
            let pointer_to_variable = lower_variable_to_value(variable.key);
            let variable_type = builder.lower_type(&variable.ty)?;
            Ok(builder.push(ir::Instr::Load((pointer_to_variable, variable_type))))
        }
        ExprKind::GlobalVariable(global_variable) => {
            let pointer = builder.push(ir::Instr::GlobalVariable(
                builder
                    .mod_builder()
                    .globals
                    .translate(global_variable.reference),
            ));

            Ok(builder.push(ir::Instr::Load((
                pointer,
                builder.lower_type(&global_variable.ty)?,
            ))))
        }
        ExprKind::DeclareAssign(declare_assign) => {
            let initial_value = builder.lower_expr(&declare_assign.value)?;

            let destination = ir::Value::Reference(ir::ValueReference {
                basicblock_id: 0,
                instruction_id: declare_assign.key.index,
            });

            builder.push(ir::Instr::Store(ir::Store {
                new_value: initial_value,
                destination: destination.clone(),
            }));

            Ok(builder.push(ir::Instr::Load((
                destination,
                builder.lower_type(&declare_assign.ty)?,
            ))))
        }
        ExprKind::BasicBinaryOperation(operation) => {
            let left = builder.lower_expr(&operation.left.expr)?;
            let right = builder.lower_expr(&operation.right.expr)?;

            lower_basic_binary_operation(
                builder,
                &operation.operator,
                ir::BinaryOperands::new(left, right),
            )
        }
        ExprKind::ShortCircuitingBinaryOperation(operation) => {
            lower_short_circuiting_binary_operation(builder, operation)
        }
        ExprKind::IntegerCast(cast_from) => integer_cast(builder, cast_from),
        ExprKind::IntegerExtend(cast_from) => integer_extend(builder, cast_from),
        ExprKind::IntegerTruncate(cast) => integer_truncate(builder, cast),
        ExprKind::FloatExtend(cast) => {
            let value = builder.lower_expr(&cast.value)?;
            let ir_type = builder.lower_type(&cast.target_type)?;
            Ok(builder.push(ir::Instr::FloatExtend(value, ir_type)))
        }
        ExprKind::FloatToInteger(cast) => {
            let value = builder.lower_expr(&cast.value)?;
            let ir_type = builder.lower_type(&cast.target_type)?;

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
                .sign(Some(builder.target()))
                .expect("integer to float must know sign");

            let value = builder.lower_expr(&cast.value)?;
            let ir_type = builder.lower_type(&cast.target_type)?;
            Ok(builder.push(ir::Instr::IntegerToFloat(value, ir_type, from_sign)))
        }
        ExprKind::Member(member) => {
            let asg::Member {
                subject,
                struct_ref: asg_struct_ref,
                index,
                field_type,
            } = &**member;

            let asg::TypeKind::Structure(_name, _struct_ref, arguments) = &subject.ty.kind else {
                todo!("member operator only supports structure types for now");
            };

            let subject_pointer = builder.lower_destination(subject)?;

            let structure = &builder.asg().structs[*asg_struct_ref];
            assert!(structure.params.len() == arguments.len());

            let mut catalog = PolyCatalog::new();
            for (name, argument) in structure.params.names().zip(arguments.iter()) {
                catalog
                    .put_type(name, &builder.unpoly(argument)?.0)
                    .expect("unique type parameter names");
            }
            let poly_recipe = catalog.bake();

            let ir_struct_ref = builder.mod_builder().structs.translate(
                *asg_struct_ref,
                poly_recipe,
                |poly_recipe| {
                    monomorphize_struct(builder.mod_builder(), *asg_struct_ref, poly_recipe)
                },
            )?;

            // Access member of structure
            let member = builder.push(ir::Instr::Member {
                subject_pointer,
                struct_type: ir::Type::Struct(ir_struct_ref),
                index: *index,
            });

            let ir_type = builder.lower_type(field_type)?;
            Ok(builder.push(ir::Instr::Load((member, ir_type))))
        }
        ExprKind::ArrayAccess(array_access) => {
            let subject = match &array_access.subject {
                asg::ArrayDestination::Expr(subject) => builder.lower_expr(subject)?,
                asg::ArrayDestination::Destination(subject) => {
                    builder.lower_destination(subject)?
                }
            };

            let index = builder.lower_expr(&array_access.index)?;
            let item_type = builder.lower_type(&array_access.item_type)?;

            let item = builder.push(ir::Instr::ArrayAccess {
                item_type: item_type.clone(),
                subject_pointer: subject,
                index,
            });

            Ok(builder.push(ir::Instr::Load((item, item_type))))
        }
        ExprKind::StructLiteral(struct_literal) => {
            let asg::StructLiteral {
                struct_type,
                fields,
            } = &**struct_literal;

            let result_ir_type = builder.lower_type(struct_type)?;
            let mut values = Vec::with_capacity(fields.len());

            // Evaluate field values in the order specified by the struct literal
            for (_, expr, index) in fields.iter() {
                let ir_value = builder.lower_expr(expr)?;
                values.push((index, ir_value));
            }

            // Sort resulting values by index
            values.sort_by(|(a, _), (b, _)| a.cmp(b));

            // Drop the index part of the values
            let values = values.drain(..).map(|(_, value)| value).collect();

            Ok(builder.push(ir::Instr::StructLiteral(result_ir_type, values)))
        }
        ExprKind::UnaryMathOperation(operation) => {
            let asg::UnaryMathOperation { operator, inner } = &**operation;
            let value = builder.lower_expr(&inner.expr)?;

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
        ExprKind::AddressOf(destination) => Ok(builder.lower_destination(destination)?),
        ExprKind::Dereference(subject) => {
            let ir_type = builder.lower_type(&subject.ty)?;

            let ir::Type::Ptr(ir_type) = ir_type else {
                panic!("Cannot lower dereference of non-pointer");
            };

            let value = builder.lower_expr(&subject.expr)?;
            Ok(builder.push(ir::Instr::Load((value, *ir_type))))
        }
        ExprKind::Conditional(conditional) => {
            let resume_basicblock_id = builder.new_block();

            let mut incoming = vec![];

            for asg::Branch { condition, block } in conditional.branches.iter() {
                let condition = builder.lower_expr(&condition.expr)?;
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
                let value = lower_stmts(builder, &block.stmts)?;

                incoming.push(ir::PhiIncoming {
                    basicblock_id: builder.current_block_id(),
                    value,
                });
                builder.continues_to(resume_basicblock_id);

                builder.use_block(false_basicblock_id);
            }

            if let Some(block) = &conditional.otherwise {
                let value = builder.lower_stmts(&block.stmts)?;

                incoming.push(ir::PhiIncoming {
                    basicblock_id: builder.current_block_id(),
                    value,
                });
            }

            builder.continues_to(resume_basicblock_id);
            builder.use_block(resume_basicblock_id);

            if conditional.otherwise.is_some() {
                if let Some(result_type) = &conditional.result_type {
                    let ir_type = builder.lower_type(result_type)?;
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

            let condition = builder.lower_expr(&while_loop.condition)?;

            builder.push(ir::Instr::ConditionalBreak(
                condition,
                ir::ConditionalBreak {
                    true_basicblock_id,
                    false_basicblock_id,
                },
            ));

            builder.use_block(true_basicblock_id);
            lower_stmts_with_break_and_continue(
                builder,
                &while_loop.block.stmts,
                Some(false_basicblock_id),
                Some(evaluate_basicblock_id),
            )?;

            builder.continues_to(evaluate_basicblock_id);

            builder.use_block(false_basicblock_id);
            Ok(Value::Literal(Literal::Void))
        }
        ExprKind::EnumMemberLiteral(enum_member_literal) => {
            let (value, ir_type, source) = match &enum_member_literal.enum_target {
                asg::EnumTarget::Named(enum_ref) => {
                    let enum_definition = &builder.asg().enums[*enum_ref];

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

                    (
                        &member.value,
                        builder.lower_type(&enum_definition.ty)?,
                        enum_definition.source,
                    )
                }
                asg::EnumTarget::Anonymous(value, ty) => {
                    (value, builder.lower_type(ty)?, enum_member_literal.source)
                }
            };

            let make_error = |_| {
                LowerErrorKind::CannotFit {
                    value: value.to_string(),
                    expected_type: enum_member_literal.human_name.to_string(),
                }
                .at(source)
            };

            Ok(match ir_type {
                ir::Type::S8 => Literal::Signed8(value.try_into().map_err(make_error)?),
                ir::Type::S16 => Literal::Signed16(value.try_into().map_err(make_error)?),
                ir::Type::S32 => Literal::Signed32(value.try_into().map_err(make_error)?),
                ir::Type::S64 => Literal::Signed64(value.try_into().map_err(make_error)?),
                ir::Type::U8 => Literal::Unsigned8(value.try_into().map_err(make_error)?),
                ir::Type::U16 => Literal::Unsigned16(value.try_into().map_err(make_error)?),
                ir::Type::U32 => Literal::Unsigned32(value.try_into().map_err(make_error)?),
                ir::Type::U64 => Literal::Unsigned64(value.try_into().map_err(make_error)?),
                _ => {
                    return Err(LowerErrorKind::EnumBackingTypeMustBeInteger {
                        enum_name: enum_member_literal.human_name.to_string(),
                    }
                    .at(source));
                }
            }
            .into())
        }
        ExprKind::ResolvedNamedExpression(resolved_expr) => builder.lower_expr(resolved_expr),
        ExprKind::Zeroed(ty) => Ok(ir::Value::Literal(Literal::Zeroed(builder.lower_type(ty)?))),
        ExprKind::SizeOf(ty, mode) => {
            Ok(builder.push(ir::Instr::SizeOf(builder.lower_type(ty)?, *mode)))
        }
        ExprKind::InterpreterSyscall(syscall, args) => {
            let mut values = Vec::with_capacity(args.len());

            for arg in args {
                values.push(builder.lower_expr(arg)?);
            }

            Ok(builder.push(ir::Instr::InterpreterSyscall(*syscall, values)))
        }
        ExprKind::Break => {
            let Some(target_basicblock_id) = builder.break_basicblock_id else {
                return Err(LowerErrorKind::Other {
                    message: "Nowhere to break to".into(),
                }
                .at(expr.source));
            };

            builder.push(ir::Instr::Break(ir::Break {
                basicblock_id: target_basicblock_id,
            }));

            Ok(ir::Value::Literal(Literal::Void))
        }
        ExprKind::Continue => {
            let Some(target_basicblock_id) = builder.continue_basicblock_id else {
                return Err(LowerErrorKind::Other {
                    message: "Nowhere to continue to".into(),
                }
                .at(expr.source));
            };

            builder.push(ir::Instr::Break(ir::Break {
                basicblock_id: target_basicblock_id,
            }));

            Ok(ir::Value::Literal(Literal::Void))
        }
        ExprKind::StaticAssert(condition, message) => {
            // TODO: How would this to make sense in generic functions?
            // It would need access to polymorphs if used, so would only
            // be able to do it when lowering (here).
            // But if the condition doen't depend at all on polymorphs,
            // then ideally we would evaluate it only once regardless
            // of how many instances of the generic function are instatiated.
            // (including zero)

            let evaluated = evaluate_const_integer_expr(builder.mod_builder(), &condition.expr)?;

            if evaluated.is_zero() {
                return Err(LowerErrorKind::StaticAssertFailed(message.clone()).at(expr.source));
            }

            Ok(ir::Value::Literal(Literal::Void))
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct EvaluatedConstInteger {
    pub signed: bool,
    pub bits: BitUnits,
    pub value: BigInt,
}

impl EvaluatedConstInteger {
    pub fn is_zero(&self) -> bool {
        self.value == BigInt::ZERO
    }
}

// NOTE: Should this be combined with the version that can happen at C parse-time?
pub fn evaluate_const_integer_expr(
    mod_builder: &ModBuilder,
    condition: &asg::Expr,
) -> Result<EvaluatedConstInteger, LowerError> {
    match &condition.kind {
        ExprKind::BooleanLiteral(value) => Ok(EvaluatedConstInteger {
            signed: false,
            bits: BitUnits::of(1),
            value: BigInt::from_u8(*value as u8).unwrap(),
        }),
        ExprKind::IntegerLiteral(_)
        | ExprKind::IntegerKnown(_)
        | ExprKind::EnumMemberLiteral(_) => todo!(),
        ExprKind::ResolvedNamedExpression(inner) => evaluate_const_integer_expr(mod_builder, inner),
        ExprKind::Variable(_)
        | ExprKind::GlobalVariable(_)
        | ExprKind::FloatingLiteral(_, _)
        | ExprKind::String(_)
        | ExprKind::NullTerminatedString(_)
        | ExprKind::Null
        | ExprKind::Call(_)
        | ExprKind::PolyCall(_)
        | ExprKind::DeclareAssign(_)
        | ExprKind::BasicBinaryOperation(_) => todo!(),
        ExprKind::ShortCircuitingBinaryOperation(_) => todo!(),
        ExprKind::IntegerCast(_) => todo!(),
        ExprKind::IntegerExtend(_) => todo!(),
        ExprKind::IntegerTruncate(_) => todo!(),
        ExprKind::FloatExtend(_)
        | ExprKind::FloatToInteger(_)
        | ExprKind::IntegerToFloat(_)
        | ExprKind::Member(_)
        | ExprKind::StructLiteral(_)
        | ExprKind::UnaryMathOperation(_) => todo!(),
        ExprKind::Dereference(_) | ExprKind::AddressOf(_) | ExprKind::Conditional(_) => todo!(),
        ExprKind::While(_) | ExprKind::ArrayAccess(_) | ExprKind::Zeroed(_) => todo!(),
        ExprKind::SizeOf(_, _)
        | ExprKind::InterpreterSyscall(_, _)
        | ExprKind::Break
        | ExprKind::Continue
        | ExprKind::StaticAssert(_, _) => Err(LowerError::other(
            "Expected constant integer expression",
            condition.source,
        )),
    }
}

pub fn lower_destination(
    builder: &mut FuncBuilder,
    destination: &Destination,
) -> Result<ir::Value, LowerError> {
    match &destination.kind {
        DestinationKind::Variable(variable) => Ok(lower_variable_to_value(variable.key)),
        DestinationKind::GlobalVariable(global_variable) => {
            let pointer = builder.push(ir::Instr::GlobalVariable(
                builder
                    .mod_builder()
                    .globals
                    .translate(global_variable.reference),
            ));
            Ok(pointer)
        }
        DestinationKind::Member {
            subject,
            struct_ref: asg_struct_ref,
            index,
            ..
        } => {
            // TODO: Combine this similar code with normal lowering?

            let subject_pointer = builder.lower_destination(subject)?;

            let asg::TypeKind::Structure(_name, _struct_ref, arguments) = &subject.ty.kind else {
                todo!("member operator only supports structure types for now");
            };

            let structure = &builder.asg().structs[*asg_struct_ref];
            assert!(structure.params.len() == arguments.len());

            let mut catalog = PolyCatalog::new();
            for (name, argument) in structure.params.names().zip(arguments.iter()) {
                catalog
                    .put_type(name, &builder.unpoly(argument)?.0)
                    .expect("unique type parameter names");
            }
            let poly_recipe = catalog.bake();

            let ir_struct_ref = builder.mod_builder().structs.translate(
                *asg_struct_ref,
                poly_recipe,
                |poly_recipe| {
                    monomorphize_struct(builder.mod_builder(), *asg_struct_ref, poly_recipe)
                },
            )?;

            Ok(builder.push(ir::Instr::Member {
                subject_pointer,
                struct_type: ir::Type::Struct(ir_struct_ref),
                index: *index,
            }))
        }
        DestinationKind::ArrayAccess(array_access) => {
            let subject_pointer = match &array_access.subject {
                asg::ArrayDestination::Expr(subject) => builder.lower_expr(subject)?,
                asg::ArrayDestination::Destination(subject) => {
                    builder.lower_destination(subject)?
                }
            };

            let index = builder.lower_expr(&array_access.index)?;
            let item_type = builder.lower_type(&array_access.item_type)?;

            Ok(builder.push(ir::Instr::ArrayAccess {
                item_type,
                subject_pointer,
                index,
            }))
        }
        DestinationKind::Dereference(pointer) => Ok(builder.lower_expr(pointer)?),
    }
}

fn lower_variable_to_value(key: VariableStorageKey) -> Value {
    Value::Reference(ValueReference {
        basicblock_id: 0,
        instruction_id: key.index,
    })
}

fn lower_add(
    builder: &mut FuncBuilder,
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
    builder: &mut FuncBuilder,
    operator: &asg::BasicBinaryOperator,
    operands: ir::BinaryOperands,
) -> Result<ir::Value, LowerError> {
    match operator {
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
            builder.target().default_float_or_sign_from_lax(mode),
        ))),
        asg::BasicBinaryOperator::Modulus(mode) => Ok(builder.push(ir::Instr::Modulus(
            operands,
            builder.target().default_float_or_sign_from_lax(mode),
        ))),
        asg::BasicBinaryOperator::Equals(mode) => {
            Ok(builder.push(ir::Instr::Equals(operands, *mode)))
        }
        asg::BasicBinaryOperator::NotEquals(mode) => {
            Ok(builder.push(ir::Instr::NotEquals(operands, *mode)))
        }
        asg::BasicBinaryOperator::LessThan(mode) => Ok(builder.push(ir::Instr::LessThan(
            operands,
            builder.target().default_float_or_sign_from_lax(mode),
        ))),
        asg::BasicBinaryOperator::LessThanEq(mode) => Ok(builder.push(ir::Instr::LessThanEq(
            operands,
            builder.target().default_float_or_sign_from_lax(mode),
        ))),
        asg::BasicBinaryOperator::GreaterThan(mode) => Ok(builder.push(ir::Instr::GreaterThan(
            operands,
            builder.target().default_float_or_sign_from_lax(mode),
        ))),
        asg::BasicBinaryOperator::GreaterThanEq(mode) => {
            Ok(builder.push(ir::Instr::GreaterThanEq(
                operands,
                builder.target().default_float_or_sign_from_lax(mode),
            )))
        }
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
                    builder.target().default_c_integer_sign(*c_integer)
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
