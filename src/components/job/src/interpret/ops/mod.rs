use super::{Registers, SyscallHandler};
use crate::{
    interpret::{Interpreter, InterpreterError, Value, ValueKind, value::Tainted},
    ir::{self, BinaryOperands, IntegerImmediate},
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
    impl_op_divmod!(div, checked_div, /, DivideByZero);
    impl_op_divmod!(rem, checked_rem, %, RemainderByZero);
    impl_op_cmp!(eq, ==);
    impl_op_cmp!(neq, !=);
    impl_op_cmp!(lt, <);
    impl_op_cmp!(lte, <=);
    impl_op_cmp!(gt, >);
    impl_op_cmp!(gte, >=);
}
