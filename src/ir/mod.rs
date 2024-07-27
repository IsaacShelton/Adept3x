use crate::data_units::ByteUnits;
use crate::resolved::{FloatOrInteger, IntegerBits, StructureRef};
use crate::target_info::TargetInfo;
use derive_more::{Deref, DerefMut, IsVariant, Unwrap};
use std::{collections::HashMap, ffi::CString};

pub use crate::resolved::{FloatOrSign, IntegerSign};
pub use crate::resolved::{FunctionRef, GlobalRef};

pub type Structures = HashMap<StructureRef, Structure>;

#[derive(Clone)]
pub struct Module {
    pub target_info: TargetInfo,
    pub functions: HashMap<FunctionRef, Function>,
    pub structures: Structures,
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
    pub abide_abi: bool,
}

#[derive(Clone, Debug)]
pub struct Structure {
    pub fields: Vec<Field>,
    pub is_packed: bool,
}

#[derive(Clone, Debug, Default)]
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
    InterpreterSyscall(InterpreterSyscallKind, Vec<Value>),
}

#[derive(Copy, Clone, Debug)]
pub enum InterpreterSyscallKind {
    Println,
    BuildAddProject,
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
    pub arguments: Box<[Value]>,
    pub variadic_argument_types: Box<[Type]>,
}

#[derive(Clone, Debug)]
pub struct Store {
    pub new_value: Value,
    pub destination: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Field {
    pub ir_type: Type,
    pub properties: FieldProperties,
}

impl Field {
    pub fn basic(ir_type: Type) -> Self {
        Self {
            ir_type,
            properties: FieldProperties::default(),
        }
    }

    pub fn ir_type(&self) -> &Type {
        &self.ir_type
    }

    pub fn is_cxx_record(&self) -> bool {
        false
    }

    pub fn as_cxx_record(&self) -> Option<CXXRecord> {
        None
    }

    pub fn is_bitfield(&self) -> bool {
        false
    }

    pub fn is_zero_length_bitfield(&self) -> bool {
        // We don't support bitfields yet, but this will need to change
        // once we do
        self.is_bitfield() && todo!("is_zero_length_bitfield")
    }

    /// Returns the maximum alignment applied to the field (or 0 if unmodified)
    pub fn get_max_alignment(&self) -> ByteUnits {
        // NOTE: We don't support using `alignas` / `_Alignas` / GNU `aligned` / MSVC declspec `align`
        // on fields yet.
        // When we do, we will need to take the maximum value assigned, and return it here.
        ByteUnits::of(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FieldProperties {
    pub is_no_unique_addr: bool,
    pub is_force_packed: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for FieldProperties {
    fn default() -> Self {
        Self {
            is_no_unique_addr: false,
            is_force_packed: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CXXRecord {}

impl CXXRecord {
    pub fn is_empty(&self) -> bool {
        todo!("is_empty for c++ records not supported yet")
    }

    pub fn is_cxx_pod(&self) -> bool {
        todo!("is_cxx_pod for c++ records not supported yet")
    }

    pub fn is_packed(&self) -> bool {
        todo!("is_packed for c++ records not supported yet")
    }
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
    Union(()),
    Structure(StructureRef),
    AnonymousComposite(TypeComposite),
    FunctionPointer,
    FixedArray(Box<FixedArray>),
    Vector(Box<Vector>),
    Complex(Box<Complex>),
    Atomic(Box<Type>),
    IncompleteArray(Box<Type>),
}

impl Type {
    pub fn is_fixed_vector(&self) -> bool {
        // NOTE: We don't support fixed vector types yet
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedArray {
    pub length: u64,
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
            Field::basic(Type::U64),
            // Value
            Field::basic(self.clone()),
        ];

        Type::AnonymousComposite(TypeComposite {
            fields: subtypes,
            is_packed: false,
        })
    }

    pub fn is_integer_like(&self) -> bool {
        matches!(
            self,
            Type::Boolean
                | Type::S8
                | Type::S16
                | Type::S32
                | Type::S64
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
        )
    }

    pub fn is_signed(&self) -> Option<bool> {
        match self {
            Type::S8 | Type::S16 | Type::S32 | Type::S64 => Some(true),
            Type::Boolean | Type::U8 | Type::U16 | Type::U32 | Type::U64 => Some(false),
            _ => None,
        }
    }

    pub fn struct_fields<'a>(&'a self, ir_module: &'a Module) -> Option<&'a [Field]> {
        match self {
            Type::Structure(structure_ref) => {
                let structure = ir_module
                    .structures
                    .get(structure_ref)
                    .expect("referenced structure to exist");

                Some(&structure.fields[..])
            }
            Type::AnonymousComposite(composite) => Some(&composite.fields[..]),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeComposite {
    pub fields: Vec<Field>,
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

#[derive(Clone, Debug, Unwrap, IsVariant)]
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
            panic!("Cannot push instruction onto basicblock when basicblock is already terminated, has already: {:#?}", self.instructions);
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
            Self::InterpreterSyscall(_, _) => false,
        }
    }
}
