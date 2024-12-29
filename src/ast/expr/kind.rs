use super::{
    ArrayAccess, BasicBinaryOperation, Call, Conditional, DeclareAssign, Expr, Integer,
    InterpreterSyscall, ShortCircuitingBinaryOperation, StaticMember, StructLiteral,
    UnaryOperation, While,
};
use crate::{ast::Privacy, name::Name, source_files::Source};
use std::ffi::CString;

#[derive(Clone, Debug)]
pub enum ExprKind {
    Variable(Name),
    Boolean(bool),
    Integer(Integer),
    Float(f64),
    Char(String),
    String(String),
    NullTerminatedString(CString),
    CharLiteral(u8),
    Call(Box<Call>),
    DeclareAssign(Box<DeclareAssign>),
    BasicBinaryOperation(Box<BasicBinaryOperation>),
    ShortCircuitingBinaryOperation(Box<ShortCircuitingBinaryOperation>),
    Member(Box<Expr>, String, Privacy),
    ArrayAccess(Box<ArrayAccess>),
    StructLiteral(Box<StructLiteral>),
    UnaryOperation(Box<UnaryOperation>),
    Conditional(Conditional),
    While(Box<While>),
    StaticMember(Box<StaticMember>),
    InterpreterSyscall(Box<InterpreterSyscall>),
}

impl ExprKind {
    pub fn at(self, source: Source) -> Expr {
        Expr::new(self, source)
    }
}
