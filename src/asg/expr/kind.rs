use crate::{
    asg::*,
    ast::{FloatSize, IntegerKnown},
    ir::InterpreterSyscallKind,
    source_files::Source,
};
use expr::poly_call::PolyCall;
use num::BigInt;
use ordered_float::NotNan;
use std::ffi::CString;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ExprKind {
    Variable(Box<Variable>),
    GlobalVariable(Box<GlobalVariable>),
    BooleanLiteral(bool),
    IntegerLiteral(BigInt),
    IntegerKnown(Box<IntegerKnown>),
    FloatingLiteral(FloatSize, Option<NotNan<f64>>),
    String(String),
    NullTerminatedString(CString),
    Call(Box<Call>),
    PolyCall(Box<PolyCall>),
    DeclareAssign(Box<DeclareAssign>),
    BasicBinaryOperation(Box<BasicBinaryOperation>),
    ShortCircuitingBinaryOperation(Box<ShortCircuitingBinaryOperation>),
    IntegerCast(Box<CastFrom>),
    IntegerExtend(Box<CastFrom>),
    IntegerTruncate(Box<Cast>),
    FloatExtend(Box<Cast>),
    FloatToInteger(Box<Cast>),
    IntegerToFloat(Box<CastFrom>),
    Member(Box<Member>),
    StructLiteral(Box<StructLiteral>),
    UnaryMathOperation(Box<UnaryMathOperation>),
    Dereference(Box<TypedExpr>),
    AddressOf(Box<Destination>),
    Conditional(Box<Conditional>),
    While(Box<While>),
    ArrayAccess(Box<ArrayAccess>),
    EnumMemberLiteral(Box<EnumMemberLiteral>),
    ResolvedNamedExpression(Box<Expr>),
    Zeroed(Box<Type>),
    InterpreterSyscall(InterpreterSyscallKind, Vec<Expr>),
}

// Make sure ExprKind doesn't accidentally become huge
const _: () = assert!(std::mem::size_of::<ExprKind>() <= 40);

impl ExprKind {
    pub fn at(self, source: Source) -> Expr {
        Expr::new(self, source)
    }
}
