mod field;
mod instr;
mod value;

use arena::{Arena, Idx, new_id_with_niche};
use attributes::SymbolOwnership;
use derivative::Derivative;
use derive_more::{Deref, DerefMut, IsVariant};
pub use field::*;
pub use instr::*;
use source_files::Source;
use target::Target;
pub use value::*;

new_id_with_niche!(StructId, u64);
new_id_with_niche!(GlobalId, u64);
new_id_with_niche!(FuncId, u64);

pub type FuncRef = Idx<FuncId, Func>;
pub type GlobalRef = Idx<GlobalId, Global>;
pub type StructRef = Idx<StructId, Struct>;

#[derive(Clone, Debug)]
pub struct Module {
    pub interpreter_entry_point: Option<FuncRef>,
    pub target: Target,
    pub funcs: Arena<FuncId, Func>,
    pub structs: Arena<StructId, Struct>,
    pub globals: Arena<GlobalId, Global>,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub mangled_name: String,
    pub params: Vec<Type>,
    pub return_type: Type,
    pub basicblocks: BasicBlocks,
    pub is_cstyle_variadic: bool,
    pub ownership: SymbolOwnership,
    pub abide_abi: bool,
}

#[derive(Clone, Debug)]
pub struct Struct {
    pub name: Option<String>,
    pub fields: Vec<Field>,
    pub is_packed: bool,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct Global {
    pub mangled_name: String,
    pub ir_type: Type,
    pub is_thread_local: bool,
    pub ownership: SymbolOwnership,
}

#[derive(Clone, Debug, Default)]
pub struct BasicBlock {
    pub instructions: Vec<Instr>,
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
            Type::Struct(struct_ref) => Some(&ir_module.structs[*struct_ref].fields[..]),
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
            panic!(
                "Cannot push instruction onto basicblock when basicblock is already terminated, has already: {:#?}",
                self.instructions
            );
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
