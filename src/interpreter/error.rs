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
}

impl Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            InterpreterError::TimedOut => "exceeded max computation time",
            InterpreterError::StackOverflow => "stack overflow",
            InterpreterError::SegfaultWrite => {
                "write segfault - tried to write to null or reserved address"
            }
            InterpreterError::SegfaultRead => {
                "read segfault - tried to read from null or reserved address"
            }
            InterpreterError::DivideByZero => "divide by zero",
            InterpreterError::RemainderByZero => "remainder by zero",
            InterpreterError::PolymorphicEntryPoint => "polymorphic entry point",
        };

        f.write_str(message)
    }
}
