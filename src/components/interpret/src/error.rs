use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum InterpreterError {
    TimedOut,
    StackOverflow,
    SegfaultWrite,
    SegfaultRead,
    DivideByZero,
    RemainderByZero,
    PolymorphicEntryPoint,
    CannotCallForeignFunction(String),
}

impl Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterError::TimedOut => write!(f, "Exceeded max computation time"),
            InterpreterError::StackOverflow => write!(f, "Stack overflow"),
            InterpreterError::SegfaultWrite => {
                write!(
                    f,
                    "Write segfault - tried to write to null or reserved address"
                )
            }
            InterpreterError::SegfaultRead => {
                write!(
                    f,
                    "Read segfault - tried to read from null or reserved address"
                )
            }
            InterpreterError::DivideByZero => write!(f, "Divide by Zero"),
            InterpreterError::RemainderByZero => write!(f, "Remainder by Zero"),
            InterpreterError::PolymorphicEntryPoint => write!(f, "Entry point is polymorphic"),
            InterpreterError::CannotCallForeignFunction(name) => {
                write!(f, "Cannot call foreign function '{}' at compile-time", name)
            }
        }
    }
}
