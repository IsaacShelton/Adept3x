use crate::ir::{FuncRef, GlobalRef, Type, value::Value};
use ast::SizeOfMode;
use primitives::{FloatOrInteger, FloatOrSign, IntegerBits, IntegerSign};

#[derive(Clone, Debug)]
pub enum Instr<'env> {
    Return(Option<Value<'env>>),
    Call(Call<'env>),
    Alloca(Type<'env>),
    Store(Store<'env>),
    Load((Value<'env>, Type<'env>)),
    Malloc(Type<'env>),
    MallocArray(Type<'env>, Value<'env>),
    Free(Value<'env>),
    SizeOf(Type<'env>, Option<SizeOfMode>),
    Parameter(u32),
    GlobalVariable(GlobalRef<'env>),
    Add(BinaryOperands<'env>, FloatOrInteger),
    Checked(OverflowOperation, BinaryOperands<'env>),
    Subtract(BinaryOperands<'env>, FloatOrInteger),
    Multiply(BinaryOperands<'env>, FloatOrInteger),
    Divide(BinaryOperands<'env>, FloatOrSign),
    Modulus(BinaryOperands<'env>, FloatOrSign),
    Equals(BinaryOperands<'env>, FloatOrInteger),
    NotEquals(BinaryOperands<'env>, FloatOrInteger),
    LessThan(BinaryOperands<'env>, FloatOrSign),
    LessThanEq(BinaryOperands<'env>, FloatOrSign),
    GreaterThan(BinaryOperands<'env>, FloatOrSign),
    GreaterThanEq(BinaryOperands<'env>, FloatOrSign),
    And(BinaryOperands<'env>),
    Or(BinaryOperands<'env>),
    BitwiseAnd(BinaryOperands<'env>),
    BitwiseOr(BinaryOperands<'env>),
    BitwiseXor(BinaryOperands<'env>),
    LeftShift(BinaryOperands<'env>),
    ArithmeticRightShift(BinaryOperands<'env>),
    LogicalRightShift(BinaryOperands<'env>),
    Bitcast(Value<'env>, Type<'env>),
    ZeroExtend(Value<'env>, Type<'env>),
    SignExtend(Value<'env>, Type<'env>),
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
