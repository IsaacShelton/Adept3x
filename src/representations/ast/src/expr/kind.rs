use super::{
    ArrayAccess, BasicBinaryOperation, Call, Conditional, DeclareAssign, Expr, Integer,
    InterpreterSyscall, ShortCircuitingBinaryOperation, StaticMemberCall, StaticMemberValue,
    StructLiteral, UnaryOperation, While,
};
use crate::{NamePath, Type};
use attributes::Privacy;
use source_files::Source;
use std::ffi::CString;

#[derive(Clone, Debug)]
pub enum ExprKind {
    Variable(NamePath),
    Boolean(bool),
    Integer(Integer),
    Float(f64),
    Char(String),
    String(String),
    NullTerminatedString(CString),
    CharLiteral(u8),
    Null,
    Call(Box<Call>),
    DeclareAssign(Box<DeclareAssign>),
    BasicBinaryOperation(Box<BasicBinaryOperation>),
    ShortCircuitingBinaryOperation(Box<ShortCircuitingBinaryOperation>),
    Member(Box<Expr>, Box<str>, Privacy),
    ArrayAccess(Box<ArrayAccess>),
    StructLiteral(Box<StructLiteral>),
    UnaryOperation(Box<UnaryOperation>),
    Conditional(Conditional),
    While(Box<While>),
    StaticMemberValue(Box<StaticMemberValue>),
    StaticMemberCall(Box<StaticMemberCall>),
    SizeOf(Box<Type>, Option<SizeOfMode>),
    SizeOfValue(Box<Expr>, Option<SizeOfMode>),
    InterpreterSyscall(Box<InterpreterSyscall>),
    Break,
    Continue,
    IntegerPromote(Box<Expr>),
    StaticAssert(Box<Expr>, Option<String>),
    Is(Box<Expr>, String),
    LabelLiteral(String),
}

impl ExprKind {
    pub fn at(self, source: Source) -> Expr {
        Expr::new(self, source)
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum SizeOfMode {
    Target,
    Compilation,
}
