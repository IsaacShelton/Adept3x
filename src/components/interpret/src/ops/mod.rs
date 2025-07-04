use super::{Registers, SyscallHandler};
use crate::{
    Interpreter,
    error::InterpreterError,
    value::{Tainted, Value, ValueKind},
};
use ir::BinaryOperands;

macro_rules! impl_op_basic {
    ($name:ident, $wrapping_name:ident, $op:tt, $bool_op:tt) => {
        pub fn $name(
            &mut self,
            operands: &BinaryOperands,
            registers: &Registers<'a>,
        ) -> Value<'a> {
            let (left, right, tainted) = self.eval_binary_ops(operands, registers);

            let literal = match left {
                ir::Literal::Void => ir::Literal::Void,
                ir::Literal::Boolean(left) => {
                    ir::Literal::Boolean(left $bool_op right.clone().unwrap_boolean())
                }
                ir::Literal::Signed8(left) => {
                    ir::Literal::Signed8(left.$wrapping_name(right.clone().unwrap_signed_8()))
                }
                ir::Literal::Signed16(left) => {
                    ir::Literal::Signed16(left.$wrapping_name(right.clone().unwrap_signed_16()))
                }
                ir::Literal::Signed32(left) => {
                    ir::Literal::Signed32(left.$wrapping_name(right.clone().unwrap_signed_32()))
                }
                ir::Literal::Signed64(left) => {
                    ir::Literal::Signed64(left.$wrapping_name(right.clone().unwrap_signed_64()))
                }
                ir::Literal::Unsigned8(left) => {
                    ir::Literal::Unsigned8(left.$wrapping_name(right.clone().unwrap_unsigned_8()))
                }
                ir::Literal::Unsigned16(left) => {
                    ir::Literal::Unsigned16(left.$wrapping_name(right.clone().unwrap_unsigned_16()))
                }
                ir::Literal::Unsigned32(left) => {
                    ir::Literal::Unsigned32(left.$wrapping_name(right.clone().unwrap_unsigned_32()))
                }
                ir::Literal::Unsigned64(left) => {
                    ir::Literal::Unsigned64(left.$wrapping_name(right.clone().unwrap_unsigned_64()))
                }
                ir::Literal::Float32(left) => {
                    ir::Literal::Float32(left $op right.clone().unwrap_float_32())
                }
                ir::Literal::Float64(left) => {
                    ir::Literal::Float64(left $op right.clone().unwrap_float_64())
                }
                ir::Literal::NullTerminatedString(_) => ir::Literal::Unsigned64(0),
                ir::Literal::Zeroed(ty) => ir::Literal::Zeroed(ty.clone()),
            };

            Value { kind: ValueKind::Literal(literal), tainted }
        }
    };
}

macro_rules! impl_op_divmod {
    ($name:ident, $checked_name:ident, $op:tt, $error_name:ident) => {
        pub fn $name(
            &mut self,
            operands: &BinaryOperands,
            registers: &Registers<'a>,
        ) -> Result<Value<'a>, InterpreterError> {
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
                ir::Literal::Signed8(left) => {
                    ir::Literal::Signed8(
                        left.$checked_name(right.clone().unwrap_signed_8()).ok_or(InterpreterError::$error_name)?
                    )
                }
                ir::Literal::Signed16(left) => {
                    ir::Literal::Signed16(
                        left.$checked_name(right.clone().unwrap_signed_16()).ok_or(InterpreterError::$error_name)?
                    )
                }
                ir::Literal::Signed32(left) => {
                    ir::Literal::Signed32(
                        left.$checked_name(right.clone().unwrap_signed_32()).ok_or(InterpreterError::$error_name)?
                    )
                }
                ir::Literal::Signed64(left) => {
                    ir::Literal::Signed64(
                        left.$checked_name(right.clone().unwrap_signed_64()).ok_or(InterpreterError::$error_name)?
                    )
                }
                ir::Literal::Unsigned8(left) => {
                    ir::Literal::Unsigned8(
                        left.$checked_name(right.clone().unwrap_unsigned_8()).ok_or(InterpreterError::$error_name)?
                    )
                }
                ir::Literal::Unsigned16(left) => {
                    ir::Literal::Unsigned16(
                        left.$checked_name(right.clone().unwrap_unsigned_16()).ok_or(InterpreterError::$error_name)?
                    )
                }
                ir::Literal::Unsigned32(left) => {
                    ir::Literal::Unsigned32(
                        left.$checked_name(right.clone().unwrap_unsigned_32()).ok_or(InterpreterError::$error_name)?
                    )
                }
                ir::Literal::Unsigned64(left) => {
                    ir::Literal::Unsigned64(
                        left.$checked_name(right.clone().unwrap_unsigned_64()).ok_or(InterpreterError::$error_name)?
                    )
                }
                ir::Literal::Float32(left) => {
                    ir::Literal::Float32(left $op right.clone().unwrap_float_32())
                }
                ir::Literal::Float64(left) => {
                    ir::Literal::Float64(left $op right.clone().unwrap_float_64())
                }
                ir::Literal::NullTerminatedString(_) => ir::Literal::Unsigned64(0),
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
        pub fn $name(
            &mut self,
            operands: &BinaryOperands,
            registers: &Registers<'a>,
        ) -> Value<'a> {
            let (left, right, tainted) = self.eval_binary_ops(operands, registers);

            let value = match left {
                ir::Literal::Void => false,
                ir::Literal::Boolean(left) => {
                    left $op right.clone().unwrap_boolean()
                }
                ir::Literal::Signed8(left) => {
                    left $op right.clone().unwrap_signed_8()
                }
                ir::Literal::Signed16(left) => {
                    left $op right.clone().unwrap_signed_16()
                }
                ir::Literal::Signed32(left) => {
                    left $op right.clone().unwrap_signed_32()
                }
                ir::Literal::Signed64(left) => {
                    left $op right.clone().unwrap_signed_64()
                }
                ir::Literal::Unsigned8(left) => {
                    left $op right.clone().unwrap_unsigned_8()
                }
                ir::Literal::Unsigned16(left) => {
                    left $op right.clone().unwrap_unsigned_16()
                }
                ir::Literal::Unsigned32(left) => {
                    left $op right.clone().unwrap_unsigned_32()
                }
                ir::Literal::Unsigned64(left) => {
                    left $op right.clone().unwrap_unsigned_64()
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

impl<'a, S: SyscallHandler> Interpreter<'a, S> {
    fn eval_into_literal(
        &self,
        registers: &Registers<'a>,
        value: &ir::Value,
    ) -> (ir::Literal, Option<Tainted>) {
        let reg = self.eval(registers, value);
        (reg.kind.unwrap_literal(), reg.tainted)
    }

    fn eval_binary_ops(
        &self,
        operands: &BinaryOperands,
        registers: &Registers<'a>,
    ) -> (ir::Literal, ir::Literal, Option<Tainted>) {
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
