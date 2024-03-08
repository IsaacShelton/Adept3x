use derive_more::{Deref, DerefMut};
use slotmap::{new_key_type, SlotMap};
use std::{collections::HashMap, ffi::CString};

pub use crate::resolved::FunctionRef;

#[derive(Clone)]
pub struct Module {
    pub functions: HashMap<FunctionRef, Function>,
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
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
}

#[derive(Clone, Debug)]
pub enum Instruction {
    Return(Option<Value>),
    Call(Call),
    Alloca(Type),
    Store(Store),
    Load((Value, Type)),
    Parameter(u32),
    Add(BinaryOperands),
    Subtract(BinaryOperands),
    Multiply(BinaryOperands),
    DivideSigned(BinaryOperands),
    DivideUnsigned(BinaryOperands),
    ModulusSigned(BinaryOperands),
    ModulusUnsigned(BinaryOperands),
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
    Composite(TypeComposite),
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

    pub fn push(&mut self, instruction: Instruction) -> Result<(), ()> {
        if self.is_terminated() {
            Err(())
        } else {
            self.instructions.push(instruction);
            Ok(())
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
            Instruction::Return(_) => true,
            Instruction::Call(_) => false,
            Instruction::Alloca(_) => false,
            Instruction::Store(_) => false,
            Instruction::Load(_) => false,
            Instruction::Parameter(_) => false,
            Instruction::Add(_) => false,
            Instruction::Subtract(_) => false,
            Instruction::Multiply(_) => false,
            Instruction::DivideSigned(_) => false,
            Instruction::DivideUnsigned(_) => false,
            Instruction::ModulusSigned(_) => false,
            Instruction::ModulusUnsigned(_) => false,
        }
    }
}
