use crate::{
    interpreter::{error::InterpreterError, value::Value, Interpreter},
    ir::{self, BinaryOperands},
};

macro_rules! impl_op_basic {
    ($name:ident, $wrapping_name:ident, $op:tt, $bool_op:tt) => {
        pub fn $name(
            &mut self,
            operands: &BinaryOperands,
            block_registers: &Vec<Vec<Value>>,
        ) -> Value {
            let left = self.eval(&block_registers, &operands.left).unwrap_literal();

            let right = self
                .eval(&block_registers, &operands.right)
                .unwrap_literal();

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

            Value::Literal(literal)
        }
    };
}

macro_rules! impl_op_divmod {
    ($name:ident, $checked_name:ident, $op:tt, $error_name:ident) => {
        pub fn $name(
            &mut self,
            operands: &BinaryOperands,
            block_registers: &Vec<Vec<Value>>,
        ) -> Result<Value, InterpreterError> {
            let left = self.eval(&block_registers, &operands.left).unwrap_literal();

            let right = self
                .eval(&block_registers, &operands.right)
                .unwrap_literal();

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

            Ok(Value::Literal(literal))
        }
    };
}

impl<'a> Interpreter<'a> {
    impl_op_basic!(add, wrapping_add, +, |);
    impl_op_basic!(sub, wrapping_sub, -, ^);
    impl_op_basic!(mul, wrapping_mul, *, &);
    impl_op_divmod!(div, checked_div, /, DivideByZero);
    impl_op_divmod!(rem, checked_rem, %, RemainderByZero);
}
