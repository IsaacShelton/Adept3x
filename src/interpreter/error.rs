#[derive(Clone, Debug)]
pub enum InterpreterError {
    TimedOut,
    StackOverflow,
    MissingEntryPoint,
    SegfaultWrite,
    SegfaultRead,
    DivideByZero,
    RemainderByZero,
}
