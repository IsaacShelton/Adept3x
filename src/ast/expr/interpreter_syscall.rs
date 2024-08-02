use super::Expr;
use crate::{ast::Type, ir::InterpreterSyscallKind};

#[derive(Clone, Debug)]
pub struct InterpreterSyscall {
    pub kind: InterpreterSyscallKind,
    pub args: Vec<(Type, Expr)>,
    pub result_type: Type,
}
