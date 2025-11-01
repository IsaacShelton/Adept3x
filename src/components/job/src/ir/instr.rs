use crate::ir::{FuncRef, GlobalRef, Type, value::Value};
use ast::SizeOfMode;
use primitives::{FloatOrInteger, FloatOrSign, IntegerBits, IntegerSign};

#[derive(Clone, Debug)]
pub enum Instr<'env> {
    Return(Option<Value<'env>>),
    Call(Call<'env>),
    Alloca(Type<'env>),
    Store(Store<'env>),
    Load {
        pointer: Value<'env>,
        pointee: Type<'env>,
    },
    Malloc(Type<'env>),
    MallocArray(Type<'env>, Value<'env>),
    Free(Value<'env>),
    SizeOf(Type<'env>, Option<SizeOfMode>),
    Parameter(u32),
    GlobalVariable(GlobalRef<'env>),
    BinOp(BinaryOperands<'env>, BinOp),
    Bitcast(Value<'env>, Type<'env>),
    Extend(Value<'env>, IntegerSign, Type<'env>),
    FloatExtend(Value<'env>, Type<'env>),
    Truncate(Value<'env>, Type<'env>),
    TruncateFloat(Value<'env>, Type<'env>),
    IntegerToPointer(Value<'env>, Type<'env>),
    PointerToInteger(Value<'env>, Type<'env>),
    FloatToInteger(Value<'env>, Type<'env>, IntegerSign),
    IntegerToFloat(Value<'env>, Type<'env>, IntegerSign),
    Member {
        struct_type: Type<'env>,
        subject_pointer: Value<'env>,
        index: usize,
    },
    ArrayAccess {
        item_type: Type<'env>,
        subject_pointer: Value<'env>,
        index: Value<'env>,
    },
    StructLiteral(Type<'env>, &'env [Value<'env>]),
    IsZero(Value<'env>, FloatOrInteger),
    IsNonZero(Value<'env>, FloatOrInteger),
    Negate(Value<'env>, FloatOrInteger),
    BitComplement(Value<'env>),
    Break(Break),
    ConditionalBreak(Value<'env>, ConditionalBreak),
    Phi(Phi<'env>),
    InterpreterSyscall(interpreter_api::Syscall, &'env [Value<'env>]),
    ExitInterpreter(Value<'env>),
}

#[derive(Clone, Debug)]
pub enum BinOp {
    Simple(BinOpSimple),
    FloatOrSign(BinOpFloatOrSign, FloatOrSign),
    FloatOrInteger(BinOpFloatOrInteger, FloatOrInteger),
    Checked(OverflowOperation),
}

#[derive(Clone, Debug)]
pub enum BinOpSimple {
    And,
    Or,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    ArithmeticRightShift,
    LogicalRightShift,
}

#[derive(Clone, Debug)]
pub enum BinOpFloatOrSign {
    Divide,
    Modulus,
    LessThan,
    LessThanEq,
    GreaterThan,
    GreaterThanEq,
}

#[derive(Clone, Debug)]
pub enum BinOpFloatOrInteger {
    Add,
    Subtract,
    Multiply,
    Equals,
    NotEquals,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct OverflowOperation {
    pub operator: OverflowOperator,
    pub sign: IntegerSign,
    pub bits: IntegerBits,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum OverflowOperator {
    Add,
    Subtract,
    Multiply,
}

#[derive(Clone, Debug)]
pub struct Phi<'env> {
    pub ir_type: Type<'env>,
    pub incoming: &'env [PhiIncoming<'env>],
}

#[derive(Clone, Debug)]
pub struct PhiIncoming<'env> {
    pub basicblock_id: usize,
    pub value: Value<'env>,
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
pub struct BinaryOperands<'env> {
    pub left: Value<'env>,
    pub right: Value<'env>,
}

impl<'env> BinaryOperands<'env> {
    pub fn new(left: Value<'env>, right: Value<'env>) -> Self {
        Self { left, right }
    }
}

#[derive(Clone, Debug)]
pub struct Call<'env> {
    pub func: FuncRef<'env>,
    pub args: &'env [Value<'env>],
    pub unpromoted_variadic_arg_types: &'env [Type<'env>],
}

#[derive(Clone, Debug)]
pub struct Store<'env> {
    pub new_value: Value<'env>,
    pub destination: Value<'env>,
}
