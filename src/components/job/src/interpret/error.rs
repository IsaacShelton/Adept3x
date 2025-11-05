use crate::ir::OverflowOperator;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum InterpreterError {
    TimedOut,
    StackOverflow,
    SegfaultWrite,
    SegfaultRead,
    DivideByZero,
    RemainderByZero,
    CannotCallForeignFunction(String),
    CheckedOperationFailed(OverflowOperator),
}

impl Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TimedOut => write!(f, "Exceeded max computation time"),
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
            Self::CheckedOperationFailed(operator) => {
                write!(f, "Checked '{}' operation went out-of-bounds", operator)
            }
        }
    }
}
