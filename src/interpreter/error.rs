#[derive(Clone, Debug)]
pub enum InterpreterError {
    TimedOut,
    StackOverflow,
    SegfaultWrite,
    SegfaultRead,
    DivideByZero,
    RemainderByZero,
}
