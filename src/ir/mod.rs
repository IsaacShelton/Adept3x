mod funcs;
mod global;
mod instr;
mod structs;
mod value;

pub use crate::asg::{FloatOrSign, GlobalVarRef, IntegerSign};
use crate::{data_units::ByteUnits, source_files::Source, target::Target};
use derivative::Derivative;
use derive_more::{Deref, DerefMut, IsVariant};
use funcs::Funcs;
pub use global::Global;
pub use instr::*;
use std::collections::HashMap;
pub use structs::Structs;
pub use value::*;

pub struct Module {
    pub target: Target,
    pub structs: Structs,
    pub globals: HashMap<GlobalVarRef, Global>,
    pub funcs: Funcs,
}

impl std::fmt::Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SlotMap {{")?;
        for ir_function in self.funcs.values() {
            ir_function.fmt(f)?;
        }
        write!(f, "SlotMap }}")?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Func {
    pub mangled_name: String,
    pub params: Vec<Type>,
    pub return_type: Type,
    pub basicblocks: BasicBlocks,
    pub is_cstyle_variadic: bool,
    pub is_foreign: bool,
    pub is_exposed: bool,
    pub abide_abi: bool,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct FuncRef {
    index: usize,
}

#[derive(Clone, Debug)]
pub struct Struct {
    pub name: Option<String>,
    pub fields: Vec<Field>,
    pub is_packed: bool,
    pub source: Source,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StructRef {
    index: usize,
}

#[derive(Clone, Debug, Default)]
pub struct BasicBlock {
    pub instructions: Vec<Instr>,
}

#[derive(Derivative, Clone, Debug)]
#[derivative(Hash, PartialEq, Eq)]
pub struct Field {
    pub ir_type: Type,
    pub properties: FieldProperties,

    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    pub source: Source,
}

impl Field {
    pub fn basic(ir_type: Type, source: Source) -> Self {
        Self {
            ir_type,
            properties: FieldProperties::default(),
            source,
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
        // NOTE: We don't support bitfields yet
        false
    }

    pub fn is_unnamed(&self) -> bool {
        self.properties.is_unnamed
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
    pub is_unnamed: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for FieldProperties {
    fn default() -> Self {
        Self {
            is_no_unique_addr: false,
            is_force_packed: false,
            is_unnamed: false,
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
    Ptr(Box<Type>),
    Bool,
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
    Struct(StructRef),
    AnonymousComposite(TypeComposite),
    FuncPtr,
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

    pub fn is_product_type(&self) -> bool {
        self.is_struct() || self.is_anonymous_composite()
    }

    pub fn has_flexible_array_member(&self) -> bool {
        // NOTE: We don't support flexible array members yet
        false
    }

    pub fn is_builtin_data(&self) -> bool {
        match self {
            Type::Bool
            | Type::S8
            | Type::S16
            | Type::S32
            | Type::S64
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::F32
            | Type::F64 => true,
            Type::Ptr(_)
            | Type::Void
            | Type::Union(_)
            | Type::Struct(_)
            | Type::AnonymousComposite(_)
            | Type::FuncPtr
            | Type::FixedArray(_)
            | Type::Vector(_)
            | Type::Complex(_)
            | Type::Atomic(_)
            | Type::IncompleteArray(_) => false,
        }
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
        Type::Ptr(Box::new(self.clone()))
    }

    pub fn is_integer_like(&self) -> bool {
        matches!(
            self,
            Type::Bool
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
            Type::Bool | Type::U8 | Type::U16 | Type::U32 | Type::U64 => Some(false),
            _ => None,
        }
    }

    pub fn struct_fields<'a>(&'a self, ir_module: &'a Module) -> Option<&'a [Field]> {
        match self {
            Type::Struct(struct_ref) => Some(&ir_module.structs.get(*struct_ref).fields[..]),
            Type::AnonymousComposite(composite) => Some(&composite.fields[..]),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Derivative)]
#[derivative(PartialEq, Eq, Hash)]
pub struct TypeComposite {
    pub fields: Vec<Field>,
    pub is_packed: bool,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub source: Source,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeFunc {
    pub params: Vec<Type>,
    pub return_type: Box<Type>,
    pub is_cstyle_variadic: bool,
}

impl Module {
    pub fn new(target: Target) -> Self {
        Self {
            target,
            funcs: Funcs::new(),
            structs: Structs::new(),
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

    pub fn push(&mut self, instruction: Instr) {
        if self.is_terminated() {
            panic!("Cannot push instruction onto basicblock when basicblock is already terminated, has already: {:#?}", self.instructions);
        } else {
            self.instructions.push(instruction);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Instr> + '_ {
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

impl Instr {
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
            Self::ArithmeticRightShift(..) => false,
            Self::LogicalRightShift(..) => false,
            Self::Bitcast(..) => false,
            Self::ZeroExtend(..) => false,
            Self::SignExtend(..) => false,
            Self::FloatExtend(..) => false,
            Self::Truncate(..) => false,
            Self::TruncateFloat(..) => false,
            Self::IntegerToPointer(..) => false,
            Self::PointerToInteger(..) => false,
            Self::FloatToInteger(..) => false,
            Self::IntegerToFloat(..) => false,
            Self::Member { .. } => false,
            Self::ArrayAccess { .. } => false,
            Self::StructLiteral(..) => false,
            Self::IsZero(..) => false,
            Self::IsNonZero(..) => false,
            Self::Negate(..) => false,
            Self::BitComplement(..) => false,
            Self::Break(..) => true,
            Self::ConditionalBreak(..) => true,
            Self::Phi(..) => false,
            Self::InterpreterSyscall(_, _) => false,
        }
    }
}
