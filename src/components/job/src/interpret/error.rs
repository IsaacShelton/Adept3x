use crate::ir::OverflowOperator;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum InterpreterError {
    TimedOut(u64),
    StackOverflow,
    SegfaultWrite,
    SegfaultRead,
    DivideByZero,
    RemainderByZero,
    CannotCallForeignFunction(String),
    CannotCallVariadicFunction,
    CheckedOperationFailed(OverflowOperator),
    MaxRecursionDepthExceeded(usize),
}

impl Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TimedOut(_steps) => {
                write!(f, "Exceeded computation limit for compile-time evaluation")
            }
            Self::StackOverflow => write!(f, "Stack overflow"),
            Self::SegfaultWrite => {
                write!(
                    f,
                    "Write segfault - tried to write to null or reserved address"
                )
            }
            Self::SegfaultRead => {
                write!(
                    f,
                    "Read segfault - tried to read from null or reserved address"
                )
            }
            Self::DivideByZero => write!(f, "Divide by Zero"),
            Self::RemainderByZero => write!(f, "Remainder by Zero"),
            Self::CannotCallForeignFunction(name) => {
                write!(f, "Cannot call foreign function '{}' at compile-time", name)
            }
            Self::CannotCallVariadicFunction => {
                write!(
                    f,
                    "Calling variadic functions at compile-time is not supported yet"
                )
            }
            Self::CheckedOperationFailed(operator) => {
                write!(f, "Checked '{}' operation went out-of-bounds", operator)
            }
            Self::MaxRecursionDepthExceeded(max_depth) => {
                write!(f, "Maximum recursion depth of {} exceeded", max_depth)
            }
        }
    }
}
