use crate::resolved::{FloatOrInteger, IntegerBits, StructureRef};
use crate::target_info::TargetInfo;
use derive_more::{Deref, DerefMut, IsVariant};
use std::{collections::HashMap, ffi::CString};

pub use crate::resolved::{FloatOrSign, IntegerSign};
pub use crate::resolved::{FunctionRef, GlobalRef};

#[derive(Clone)]
pub struct Module {
    pub target_info: TargetInfo,
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
    Malloc(Type),
    MallocArray(Type, Value),
    Free(Value),
    SizeOf(Type),
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
    RightShift(BinaryOperands),
    LogicalRightShift(BinaryOperands),
    Bitcast(Value, Type),
    ZeroExtend(Value, Type),
    SignExtend(Value, Type),
    FloatExtend(Value, Type),
    Truncate(Value, Type),
    TruncateFloat(Value, Type),
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
    pub new_value: Value,
    pub destination: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, IsVariant, Hash)]
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
    FunctionPointer,
    FixedArray(Box<FixedArray>),
    Vector(Box<Vector>),
    Complex(Box<Complex>),
    Atomic(Box<Type>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedArray {
    pub size: u64,
    pub inner: Type,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Vector {
    pub element_type: Type,
    pub num_elements: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Complex {
    pub element_type: Type,
}

impl Type {
    pub fn pointer(&self) -> Self {
        Type::Pointer(Box::new(self.clone()))
    }

    pub fn reference_counted_pointer(&self) -> Self {
        // Don't allow wrapping pointer values with reference counting
        // This will catch us if we accidentally nest more than once
        assert!(!self.is_pointer());

        Type::Pointer(Box::new(self.reference_counted_no_pointer()))
    }

    pub fn reference_counted_no_pointer(&self) -> Self {
        let subtypes = vec![
            // Reference count
            Type::U64,
            // Value
            self.clone(),
        ];

        Type::AnonymousComposite(TypeComposite {
            subtypes,
            is_packed: false,
        })
    }

    pub fn is_integer_like(&self) -> bool {
        match self {
            Type::Boolean
            | Type::S8
            | Type::S16
            | Type::S32
            | Type::S64
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64 => true,
            _ => false,
        }
    }

    pub fn is_signed(&self) -> Option<bool> {
        match self {
            Type::S8 | Type::S16 | Type::S32 | Type::S64 => Some(true),
            Type::Boolean | Type::U8 | Type::U16 | Type::U32 | Type::U64 => Some(false),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeComposite {
    pub subtypes: Vec<Type>,
    pub is_packed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeFunction {
    pub parameters: Vec<Type>,
    pub return_type: Box<Type>,
    pub is_cstyle_variadic: bool,
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
    Zeroed(Type),
}

#[derive(Clone, Debug)]
pub struct ValueReference {
    pub basicblock_id: usize,
    pub instruction_id: usize,
}

impl Module {
    pub fn new(target_info: TargetInfo) -> Self {
        Self {
            target_info,
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
            Self::Return(..) => true,
            Self::Call(..) => false,
            Self::Alloca(..) => false,
            Self::Store(..) => false,
            Self::Load(..) => false,
            Self::Malloc(..) => false,
            Self::MallocArray(..) => false,
            Self::Free(..) => false,
            Self::SizeOf(..) => false,
            Self::Parameter(..) => false,
            Self::GlobalVariable(..) => false,
            Self::Add(..) => false,
            Self::Checked(..) => false,
            Self::Subtract(..) => false,
            Self::Multiply(..) => false,
            Self::Divide(..) => false,
            Self::Modulus(..) => false,
            Self::Equals(..) => false,
            Self::NotEquals(..) => false,
            Self::LessThan(..) => false,
            Self::LessThanEq(..) => false,
            Self::GreaterThan(..) => false,
            Self::GreaterThanEq(..) => false,
            Self::And(..) => false,
            Self::Or(..) => false,
            Self::BitwiseAnd(..) => false,
            Self::BitwiseOr(..) => false,
            Self::BitwiseXor(..) => false,
            Self::LeftShift(..) => false,
            Self::RightShift(..) => false,
            Self::LogicalRightShift(..) => false,
            Self::Bitcast(..) => false,
            Self::ZeroExtend(..) => false,
            Self::SignExtend(..) => false,
            Self::FloatExtend(..) => false,
            Self::Truncate(..) => false,
            Self::TruncateFloat(..) => false,
            Self::Member { .. } => false,
            Self::ArrayAccess { .. } => false,
            Self::StructureLiteral(..) => false,
            Self::IsZero(..) => false,
            Self::IsNotZero(..) => false,
            Self::Negate(..) => false,
            Self::NegateFloat(..) => false,
            Self::BitComplement(..) => false,
            Self::Break(..) => true,
            Self::ConditionalBreak(..) => true,
            Self::Phi(..) => false,
        }
    }
}
