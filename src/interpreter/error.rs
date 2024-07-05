#[derive(Clone, Debug)]
pub enum InterpreterError {
    TimedOut,
    StackOverflow,
    MissingMainFunction,
    SegfaultWrite,
    SegfaultRead,
    DivideByZero,
    RemainderByZero,
}
