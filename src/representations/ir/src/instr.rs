use crate::{FuncRef, GlobalRef, Type, value::Value};
use ast::SizeofMode;
use primitives::{FloatOrInteger, FloatOrSign, IntegerBits, IntegerSign};

#[derive(Clone, Debug)]
pub enum Instr {
    Return(Option<Value>),
    Call(Call),
    Alloca(Type),
    Store(Store),
    Load((Value, Type)),
    Malloc(Type),
    MallocArray(Type, Value),
    Free(Value),
    SizeOf(Type, Option<SizeofMode>),
    Parameter(u32),
    GlobalVariable(GlobalRef),
    Add(BinaryOperands, FloatOrInteger),
    Checked(OverflowOperation, BinaryOperands),
    Subtract(BinaryOperands, FloatOrInteger),
    Multiply(BinaryOperands, FloatOrInteger),
    Divide(BinaryOperands, FloatOrSign),
    Modulus(BinaryOperands, FloatOrSign),
    Equals(BinaryOperands, FloatOrInteger),
    NotEquals(BinaryOperands, FloatOrInteger),
    LessThan(BinaryOperands, FloatOrSign),
    LessThanEq(BinaryOperands, FloatOrSign),
    GreaterThan(BinaryOperands, FloatOrSign),
    GreaterThanEq(BinaryOperands, FloatOrSign),
    And(BinaryOperands),
    Or(BinaryOperands),
    BitwiseAnd(BinaryOperands),
    BitwiseOr(BinaryOperands),
    BitwiseXor(BinaryOperands),
    LeftShift(BinaryOperands),
    ArithmeticRightShift(BinaryOperands),
    LogicalRightShift(BinaryOperands),
    Bitcast(Value, Type),
    ZeroExtend(Value, Type),
    SignExtend(Value, Type),
    FloatExtend(Value, Type),
    Truncate(Value, Type),
    TruncateFloat(Value, Type),
    IntegerToPointer(Value, Type),
    PointerToInteger(Value, Type),
    FloatToInteger(Value, Type, IntegerSign),
    IntegerToFloat(Value, Type, IntegerSign),
    Member {
        struct_type: Type,
        subject_pointer: Value,
        index: usize,
    },
    ArrayAccess {
        item_type: Type,
        subject_pointer: Value,
        index: Value,
    },
    StructLiteral(Type, Vec<Value>),
    IsZero(Value, FloatOrInteger),
    IsNonZero(Value, FloatOrInteger),
    Negate(Value, FloatOrInteger),
    BitComplement(Value),
    Break(Break),
    ConditionalBreak(Value, ConditionalBreak),
    Phi(Phi),
    InterpreterSyscall(interpreter_api::Syscall, Vec<Value>),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum OverflowOperator {
    Add,
    Subtract,
    Multiply,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct OverflowOperation {
    pub operator: OverflowOperator,
    pub sign: IntegerSign,
    pub bits: IntegerBits,
}

#[derive(Clone, Debug)]
pub struct Phi {
    pub ir_type: Type,
    pub incoming: Vec<PhiIncoming>,
}

#[derive(Clone, Debug)]
pub struct PhiIncoming {
    pub basicblock_id: usize,
    pub value: Value,
}

#[derive(Clone, Debug)]
pub struct Break {
    pub basicblock_id: usize,
}

#[derive(Clone, Debug)]
pub struct ConditionalBreak {
    pub true_basicblock_id: usize,
    pub false_basicblock_id: usize,
}

#[derive(Clone, Debug)]
pub struct BinaryOperands {
    pub left: Value,
    pub right: Value,
}

impl BinaryOperands {
    pub fn new(left: Value, right: Value) -> Self {
        Self { left, right }
    }
}

#[derive(Clone, Debug)]
pub struct Call {
    pub func: FuncRef,
    pub args: Box<[Value]>,
    pub unpromoted_variadic_arg_types: Box<[Type]>,
}

#[derive(Clone, Debug)]
pub struct Store {
    pub new_value: Value,
    pub destination: Value,
}
