use crate::{Expr, Type};

#[derive(Clone, Debug)]
pub struct InterpreterSyscall {
    pub kind: interpreter_api::Syscall,
    pub args: Vec<(Type, Expr)>,
    pub result_type: Type,
}
