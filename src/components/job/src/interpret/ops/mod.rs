use super::{Registers, SyscallHandler};
use crate::{
    interpret::{Interpreter, InterpreterError, Value, ValueKind, value::Tainted},
    ir::{self, BinaryOperands, IntegerImmediate, OverflowOperation},
};
use primitives::{IntegerConstant, IntegerSign};

macro_rules! impl_op_basic {
    ($name:ident, $wrapping_name:ident, $op:tt, $bool_op:tt) => {
        pub fn $name<'a>(
            &mut self,
            operands: &'a BinaryOperands<'env>,
            registers: &'a Registers<'env>,
        ) -> Value<'env> {
            let (left, right, tainted) = self.eval_binary_ops(operands, registers);

            let literal = match left {
                ir::Literal::Void => ir::Literal::Void,
                ir::Literal::Boolean(left) => {
                    ir::Literal::Boolean(left $bool_op right.clone().unwrap_boolean())
                }
                ir::Literal::Integer(immediate) => {
                    let register: u64 = match immediate.value() {
                        IntegerConstant::Signed(left) => left.$wrapping_name(right.unwrap_signed()) as u64,
                        IntegerConstant::Unsigned(left) => left.$wrapping_name(right.unwrap_unsigned()),
                    } & immediate.mask();

                    ir::Literal::Integer(
                        IntegerImmediate::new(
                            match immediate.value().sign() {
                                IntegerSign::Signed => IntegerConstant::Signed(register as i64),
                                IntegerSign::Unsigned => IntegerConstant::Unsigned(register as u64),
                            },
                            immediate.bits(),
                        )
                        .expect("interpreter operation to remain inbounds of integer type"),
                    )
                }
                ir::Literal::Float32(left) => {
                    ir::Literal::Float32(left $op right.clone().unwrap_float_32())
                }
                ir::Literal::Float64(left) => {
                    ir::Literal::Float64(left $op right.clone().unwrap_float_64())
                }
                ir::Literal::NullTerminatedString(_) => ir::Literal::new_u64(0),
                ir::Literal::Zeroed(ty) => ir::Literal::Zeroed(ty),
            };

            Value { kind: ValueKind::Literal(literal), tainted }
        }
    };
}

macro_rules! impl_op_checked {
    ($name:ident, $checked_name:ident) => {
        pub fn $name<'a>(
            &mut self,
            operands: &'a BinaryOperands<'env>,
            registers: &'a Registers<'env>,
            operation: &'a OverflowOperation,
        ) -> Result<Value<'env>, InterpreterError> {
            let (left, right, tainted) = self.eval_binary_ops(operands, registers);

            let overflow_operator = &operation.operator;
            let integer_bits = operation.bits;

            let literal = match left {
                ir::Literal::Integer(immediate) => {
                    // TODO: CLEANUP: Clean up this part
                    let integer_constant = match immediate.value() {
                        IntegerConstant::Signed(left) => {
                            let right = right.unwrap_signed();

                            let checked_result = left
                                .$checked_name(right)
                                .into_iter()
                                .flat_map(|x| {
                                    (integer_bits.min_signed() <= x
                                        && x <= integer_bits.max_signed())
                                    .then_some(x)
                                })
                                .next()
                                .ok_or(InterpreterError::CheckedOperationFailed(
                                    *overflow_operator,
                                ))?;

                            IntegerConstant::Signed(checked_result)
                        }
                        IntegerConstant::Unsigned(left) => {
                            let right = right.unwrap_unsigned();

                            let checked_result = left
                                .$checked_name(right)
                                .into_iter()
                                .flat_map(|x| {
                                    (integer_bits.min_unsigned() <= x
                                        && x <= integer_bits.max_unsigned())
                                    .then_some(x)
                                })
                                .next()
                                .ok_or(InterpreterError::CheckedOperationFailed(
                                    *overflow_operator,
                                ))?;

                            IntegerConstant::Unsigned(checked_result)
                        }
                    };

                    ir::Literal::Integer(
                        IntegerImmediate::new(integer_constant, immediate.bits())
                            .expect("interpreter operation to remain inbounds of integer type"),
                    )
                }
                _ => ir::Literal::Void,
            };

            Ok(Value {
                kind: ValueKind::Literal(literal),
                tainted,
            })
        }
    };
}

macro_rules! impl_op_divmod {
    ($name:ident, $checked_name:ident, $op:tt, $error_name:ident) => {
        pub fn $name<'a>(
            &mut self,
            operands: &'a BinaryOperands<'env>,
            registers: &'a Registers<'env>,
        ) -> Result<Value<'env>, InterpreterError> {
            let (left, right, tainted) = self.eval_binary_ops(operands, registers);

            let literal = match left {
                ir::Literal::Void => ir::Literal::Void,
                ir::Literal::Boolean(left) => {
                    if right.clone().unwrap_boolean() {
                        ir::Literal::Boolean(left)
                    } else {
                        return Err(InterpreterError::$error_name);
                    }
                }
                ir::Literal::Integer(immediate) => {
                    let register: u64 = match immediate.value() {
                        IntegerConstant::Signed(left) => left.$checked_name(right.unwrap_signed()).ok_or(InterpreterError::$error_name)? as u64,
                        IntegerConstant::Unsigned(left) => left.$checked_name(right.unwrap_unsigned()).ok_or(InterpreterError::$error_name)?,
                    } & immediate.mask();

                    ir::Literal::Integer(
                        IntegerImmediate::new(
                            match immediate.value().sign() {
                                IntegerSign::Signed => IntegerConstant::Signed(register as i64),
                                IntegerSign::Unsigned => IntegerConstant::Unsigned(register as u64),
                            },
                            immediate.bits(),
                        )
                        .expect("checked interpreter operation to remain inbounds of integer type"),
                    )
                }
                ir::Literal::Float32(left) => {
                    ir::Literal::Float32(left $op right.clone().unwrap_float_32())
                }
                ir::Literal::Float64(left) => {
                    ir::Literal::Float64(left $op right.clone().unwrap_float_64())
                }
                ir::Literal::NullTerminatedString(_) => ir::Literal::new_u64(0),
                ir::Literal::Zeroed(_) => return Err(InterpreterError::$error_name),
            };

            Ok(Value{
                kind: ValueKind::Literal(literal),
                tainted
            })
        }
    };
}

macro_rules! impl_op_cmp {
    ($name:ident, $op:tt) => {
        pub fn $name<'a>(
            &mut self,
            operands: &'a BinaryOperands<'env>,
            registers: &'a Registers<'env>,
        ) -> Value<'env> {
            let (left, right, tainted) = self.eval_binary_ops(operands, registers);

            let value = match left {
                ir::Literal::Void => false,
                ir::Literal::Boolean(left) => {
                    left $op right.clone().unwrap_boolean()
                }
                ir::Literal::Integer(immediate) => {
                    match immediate.value() {
                        IntegerConstant::Signed(left) => left $op right.unwrap_signed(),
                        IntegerConstant::Unsigned(left) => left $op right.unwrap_unsigned(),
                    }
                }
                ir::Literal::Float32(left) => {
                    left $op right.clone().unwrap_float_32()
                }
                ir::Literal::Float64(left) => {
                    left $op right.clone().unwrap_float_64()
                }
                ir::Literal::NullTerminatedString(_) => false,
                ir::Literal::Zeroed(_) => true,
            };

            Value {
                kind: ValueKind::Literal(ir::Literal::Boolean(value)),
                tainted,
            }
        }
    };
}

macro_rules! impl_op_bitwise {
    ($name:ident, $bitwise_op:tt, $bool_op:tt) => {
        pub fn $name<'a>(
            &mut self,
            operands: &'a BinaryOperands<'env>,
            registers: &'a Registers<'env>,
        ) -> Value<'env> {
            let (left, right, tainted) = self.eval_binary_ops(operands, registers);

            // TODO: Clean this up, we really shouldn't need to rely on the type of the data
            let literal = if left.is_boolean() || right.is_boolean() {
                let l = left.unwrap_boolean();
                let r = right.unwrap_boolean();
                ir::Literal::Boolean(l $bool_op r)
            } else {
                let properties = left.unwrap_integer();
                let l = left.unwrap_integer().value().raw_data();
                let r = right.unwrap_integer().value().raw_data();
                let raw_result = l $bitwise_op r;
                let bits = properties.bits();
                let sign = properties.value().sign();
                ir::Literal::Integer(
                    IntegerImmediate::new(IntegerConstant::from_raw_data(raw_result, sign), bits)
                        .expect("bitwise operation result to fit within same integer range"),
                )
            };

            Value {
                kind: ValueKind::Literal(literal),
                tainted,
            }
        }
    };
}

impl<'env, S: SyscallHandler> Interpreter<'env, S> {
    fn eval_into_literal<'a>(
        &self,
        registers: &'a Registers<'env>,
        value: &'a ir::Value<'env>,
    ) -> (ir::Literal<'env>, Option<Tainted>) {
        let reg = self.eval(registers, value);
        (reg.kind.unwrap_literal(), reg.tainted)
    }

    fn eval_binary_ops<'a>(
        &self,
        operands: &'a BinaryOperands<'env>,
        registers: &'a Registers<'env>,
    ) -> (ir::Literal<'env>, ir::Literal<'env>, Option<Tainted>)
    where
        'env: 'a,
    {
        let (left, l_tainted) = self.eval_into_literal(registers, &operands.left);
        let (right, r_tainted) = self.eval_into_literal(registers, &operands.right);
        (left, right, l_tainted.or(r_tainted))
    }

    impl_op_basic!(add, wrapping_add, +, |);
    impl_op_basic!(sub, wrapping_sub, -, ^);
    impl_op_basic!(mul, wrapping_mul, *, &);
    impl_op_checked!(checked_add, checked_add);
    impl_op_checked!(checked_sub, checked_sub);
    impl_op_checked!(checked_mul, checked_mul);
    impl_op_divmod!(div, checked_div, /, DivideByZero);
    impl_op_divmod!(rem, checked_rem, %, RemainderByZero);
    impl_op_cmp!(eq, ==);
    impl_op_cmp!(neq, !=);
    impl_op_cmp!(lt, <);
    impl_op_cmp!(lte, <=);
    impl_op_cmp!(gt, >);
    impl_op_cmp!(gte, >=);
    impl_op_bitwise!(bitwise_and, &, &&);
    impl_op_bitwise!(bitwise_or, &, &&);
    impl_op_bitwise!(bitwise_xor, ^, ^);
}
