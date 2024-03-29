use derive_more::{Deref, DerefMut};
use std::{collections::HashMap, ffi::CString};

pub use crate::resolved::{FunctionRef, GlobalRef};
use crate::resolved::{IntegerBits, StructureRef};

#[derive(Clone)]
pub struct Module {
    pub functions: HashMap<FunctionRef, Function>,
    pub structures: HashMap<StructureRef, Structure>,
    pub globals: HashMap<GlobalRef, Global>,
}

impl std::fmt::Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SlotMap {{")?;
        for ir_function in self.functions.values() {
            ir_function.fmt(f)?;
        }
        write!(f, "SlotMap }}")?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Global {
    pub mangled_name: String,
    pub ir_type: Type,
    pub is_foreign: bool,
    pub is_thread_local: bool,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub mangled_name: String,
    pub parameters: Vec<Type>,
    pub return_type: Type,
    pub basicblocks: BasicBlocks,
    pub is_cstyle_variadic: bool,
    pub is_foreign: bool,
    pub is_exposed: bool,
    pub variables: Vec<Value>,
}

#[derive(Clone, Debug)]
pub struct Structure {
    pub fields: Vec<Type>,
    pub is_packed: bool,
}

#[derive(Clone, Debug)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
}

pub use crate::resolved::IntegerSign;

#[derive(Clone, Debug)]
pub enum FloatOrSign {
    Integer(IntegerSign),
    Float,
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
pub enum Instruction {
    Return(Option<Value>),
    Call(Call),
    Alloca(Type),
    Store(Store),
    Load((Value, Type)),
    Parameter(u32),
    GlobalVariable(GlobalRef),
    Add(BinaryOperands),
    Checked(OverflowOperation, BinaryOperands),
    Subtract(BinaryOperands),
    Multiply(BinaryOperands),
    Divide(BinaryOperands, IntegerSign),
    Modulus(BinaryOperands, IntegerSign),
    Equals(BinaryOperands),
    NotEquals(BinaryOperands),
    LessThan(BinaryOperands, IntegerSign),
    LessThanEq(BinaryOperands, IntegerSign),
    GreaterThan(BinaryOperands, IntegerSign),
    GreaterThanEq(BinaryOperands, IntegerSign),
    And(BinaryOperands),
    Or(BinaryOperands),
    BitwiseAnd(BinaryOperands),
    BitwiseOr(BinaryOperands),
    BitwiseXor(BinaryOperands),
    LeftShift(BinaryOperands),
    RightShift(BinaryOperands),
    LogicalRightShift(BinaryOperands),
    Bitcast(Value, Type),
    ZeroExtend(Value, Type),
    SignExtend(Value, Type),
    FloatExtend(Value, Type),
    Truncate(Value, Type),
    TruncateFloat(Value, Type),
    Member(Value, StructureRef, usize),
    StructureLiteral(Type, Vec<Value>),
    IsZero(Value),
    IsNotZero(Value),
    Negate(Value),
    NegateFloat(Value),
    BitComplement(Value),
    Break(Break),
    ConditionalBreak(Value, ConditionalBreak),
    Phi(Phi),
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
    pub function: FunctionRef,
    pub arguments: Vec<Value>,
}

#[derive(Clone, Debug)]
pub struct Store {
    pub source: Value,
    pub destination: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Pointer(Box<Type>),
    Boolean,
    S8,
    S16,
    S32,
    S64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Void,
    Structure(StructureRef),
    AnonymousComposite(TypeComposite),
    Function(TypeFunction),
    UntypedEnum(TypeUntypedEnum),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeComposite {
    pub subtypes: Vec<Type>,
    pub is_packed: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TypeFunction {
    pub parameters: Vec<Type>,
    pub return_type: Box<Type>,
    pub is_cstyle_variadic: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeUntypedEnum {
    pub member: String,
}

#[derive(Clone, Debug)]
pub enum Value {
    Literal(Literal),
    Reference(ValueReference),
}

#[derive(Clone, Debug)]
pub enum Literal {
    Void,
    Boolean(bool),
    Signed8(i8),
    Signed16(i16),
    Signed32(i32),
    Signed64(i64),
    Unsigned8(u8),
    Unsigned16(u16),
    Unsigned32(u32),
    Unsigned64(u64),
    Float32(f32),
    Float64(f64),
    NullTerminatedString(CString),
}

#[derive(Clone, Debug)]
pub struct ValueReference {
    pub basicblock_id: usize,
    pub instruction_id: usize,
}

impl Module {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            structures: HashMap::new(),
            globals: HashMap::new(),
        }
    }
}

impl BasicBlock {
    pub fn new() -> Self {
        Self {
            instructions: vec![],
        }
    }

    pub fn is_terminated(&self) -> bool {
        self.instructions
            .last()
            .map(|instruction| instruction.is_terminating())
            .unwrap_or(false)
    }

    pub fn push(&mut self, instruction: Instruction) {
        if self.is_terminated() {
            panic!("Cannot push instruction onto basicblock when basicblock is already terminated");
        } else {
            self.instructions.push(instruction);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Instruction> + '_ {
        self.instructions.iter()
    }
}

#[derive(Clone, Debug, Deref, DerefMut, Default)]
pub struct BasicBlocks {
    #[deref]
    #[deref_mut]
    pub blocks: Vec<BasicBlock>,
}

impl BasicBlocks {
    pub fn new() -> Self {
        Self { blocks: vec![] }
    }

    pub fn is_terminated(&self) -> bool {
        if let Some(basicblock) = self.blocks.last() {
            basicblock.is_terminated()
        } else {
            false
        }
    }
}

impl Instruction {
    pub fn is_terminating(&self) -> bool {
        match self {
            Instruction::Return(..) => true,
            Instruction::Call(..) => false,
            Instruction::Alloca(..) => false,
            Instruction::Store(..) => false,
            Instruction::Load(..) => false,
            Instruction::Parameter(..) => false,
            Instruction::GlobalVariable(..) => false,
            Instruction::Add(..) => false,
            Instruction::Checked(..) => false,
            Instruction::Subtract(..) => false,
            Instruction::Multiply(..) => false,
            Instruction::Divide(..) => false,
            Instruction::Modulus(..) => false,
            Instruction::Equals(..) => false,
            Instruction::NotEquals(..) => false,
            Instruction::LessThan(..) => false,
            Instruction::LessThanEq(..) => false,
            Instruction::GreaterThan(..) => false,
            Instruction::GreaterThanEq(..) => false,
            Instruction::And(..) => false,
            Instruction::Or(..) => false,
            Instruction::BitwiseAnd(..) => false,
            Instruction::BitwiseOr(..) => false,
            Instruction::BitwiseXor(..) => false,
            Instruction::LeftShift(..) => false,
            Instruction::RightShift(..) => false,
            Instruction::LogicalRightShift(..) => false,
            Instruction::Bitcast(..) => false,
            Instruction::ZeroExtend(..) => false,
            Instruction::SignExtend(..) => false,
            Instruction::FloatExtend(..) => false,
            Instruction::Truncate(..) => false,
            Instruction::TruncateFloat(..) => false,
            Instruction::Member(..) => false,
            Instruction::StructureLiteral(..) => false,
            Instruction::IsZero(..) => false,
            Instruction::IsNotZero(..) => false,
            Instruction::Negate(..) => false,
            Instruction::NegateFloat(..) => false,
            Instruction::BitComplement(..) => false,
            Instruction::Break(..) => true,
            Instruction::ConditionalBreak(..) => true,
            Instruction::Phi(..) => false,
        }
    }
}
