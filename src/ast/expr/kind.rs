use super::{
    ArrayAccess, BasicBinaryOperation, Call, Conditional, DeclareAssign, EnumMemberLiteral, Expr,
    Integer, InterpreterSyscall, ShortCircuitingBinaryOperation, StructureLiteral, UnaryOperation,
    While,
};
use crate::{name::Name, source_files::Source};
use std::ffi::CString;

#[derive(Clone, Debug)]
pub enum ExprKind {
    Variable(Name),
    Boolean(bool),
    Integer(Integer),
    Float(f64),
    String(String),
    NullTerminatedString(CString),
    Call(Box<Call>),
    DeclareAssign(Box<DeclareAssign>),
    BasicBinaryOperation(Box<BasicBinaryOperation>),
    ShortCircuitingBinaryOperation(Box<ShortCircuitingBinaryOperation>),
    Member(Box<Expr>, String),
    ArrayAccess(Box<ArrayAccess>),
    StructureLiteral(Box<StructureLiteral>),
    UnaryOperation(Box<UnaryOperation>),
    Conditional(Conditional),
    While(Box<While>),
    EnumMemberLiteral(Box<EnumMemberLiteral>),
    InterpreterSyscall(Box<InterpreterSyscall>),
}

impl ExprKind {
    pub fn at(self, source: Source) -> Expr {
        Expr::new(self, source)
    }
}
