mod variable_storage;

use crate::{ast::Source, source_file_cache::SourceFileCache};
use derive_more::{IsVariant, Unwrap};
use indexmap::IndexMap;
use num_bigint::BigInt;
use num_traits::Zero;
use slotmap::{new_key_type, SlotMap};
use std::{
    ffi::CString,
    fmt::{Debug, Display},
};

pub use self::variable_storage::VariableStorageKey;
pub use crate::ast::UnaryOperator;
pub use crate::ast::{FloatSize, IntegerBits, IntegerSign};
pub use variable_storage::VariableStorage;

new_key_type! {
    pub struct FunctionRef;
    pub struct GlobalRef;
    pub struct StructureRef;
}

#[derive(Clone, Debug)]
pub struct Ast<'a> {
    pub source_file_cache: &'a SourceFileCache,
    pub entry_point: Option<FunctionRef>,
    pub functions: SlotMap<FunctionRef, Function>,
    pub structures: SlotMap<StructureRef, Structure>,
    pub globals: SlotMap<GlobalRef, Global>,
}

impl<'a> Ast<'a> {
    pub fn new(source_file_cache: &'a SourceFileCache) -> Self {
        Self {
            source_file_cache,
            entry_point: None,
            functions: SlotMap::with_key(),
            structures: SlotMap::with_key(),
            globals: SlotMap::with_key(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Global {
    pub name: String,
    pub resolved_type: Type,
    pub source: Source,
    pub is_foreign: bool,
    pub is_thread_local: bool,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub parameters: Parameters,
    pub return_type: Type,
    pub stmts: Vec<Stmt>,
    pub is_foreign: bool,
    pub variables: VariableStorage,
}

#[derive(Clone, Debug)]
pub struct Parameters {
    pub required: Vec<Parameter>,
    pub is_cstyle_vararg: bool,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            required: vec![],
            is_cstyle_vararg: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: String,
    pub resolved_type: Type,
}

#[derive(Clone, Debug)]
pub struct Structure {
    pub name: String,
    pub fields: IndexMap<String, Field>,
    pub is_packed: bool,
}

pub use crate::ast::Privacy;

#[derive(Clone, Debug)]
pub struct Field {
    pub resolved_type: Type,
    pub privacy: Privacy,
}

#[derive(Clone, Debug, PartialEq, IsVariant)]
pub enum Type {
    Boolean,
    Integer {
        bits: IntegerBits,
        sign: IntegerSign,
    },
    IntegerLiteral(BigInt),
    FloatLiteral(f64),
    Float(FloatSize),
    Pointer(Box<Type>),
    PlainOldData(String, StructureRef),
    Void,
    ManagedStructure(String, StructureRef),
}

impl Type {
    pub fn sign(&self) -> Option<IntegerSign> {
        match self {
            Type::Boolean => None,
            Type::Integer { sign, .. } => Some(*sign),
            Type::IntegerLiteral(value) => Some(if value >= &BigInt::zero() {
                IntegerSign::Unsigned
            } else {
                IntegerSign::Signed
            }),
            Type::Float(_) => None,
            Type::FloatLiteral(_) => None,
            Type::Pointer(_) => None,
            Type::PlainOldData(_, _) => None,
            Type::Void => None,
            Type::ManagedStructure(_, _) => None,
        }
    }

    pub fn is_void_pointer(&self) -> bool {
        match self {
            Type::Pointer(inner) if inner.is_void() => true,
            _ => false,
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Boolean => {
                write!(f, "bool")?;
            }
            Type::Integer { bits, sign } => {
                f.write_str(match (bits, sign) {
                    (IntegerBits::Normal, IntegerSign::Signed) => "int",
                    (IntegerBits::Normal, IntegerSign::Unsigned) => "uint",
                    (IntegerBits::Bits8, IntegerSign::Signed) => "i8",
                    (IntegerBits::Bits8, IntegerSign::Unsigned) => "u8",
                    (IntegerBits::Bits16, IntegerSign::Signed) => "i16",
                    (IntegerBits::Bits16, IntegerSign::Unsigned) => "u16",
                    (IntegerBits::Bits32, IntegerSign::Signed) => "i32",
                    (IntegerBits::Bits32, IntegerSign::Unsigned) => "u32",
                    (IntegerBits::Bits64, IntegerSign::Signed) => "i64",
                    (IntegerBits::Bits64, IntegerSign::Unsigned) => "u64",
                })?;
            }
            Type::IntegerLiteral(value) => {
                write!(f, "integer {}", value)?;
            }
            Type::Float(size) => match size {
                FloatSize::Normal => f.write_str("float")?,
                FloatSize::Bits32 => f.write_str("f32")?,
                FloatSize::Bits64 => f.write_str("f64")?,
            },
            Type::FloatLiteral(value) => write!(f, "float {}", value)?,
            Type::Pointer(inner) => {
                write!(f, "ptr<{}>", inner)?;
            }
            Type::PlainOldData(name, _) => {
                write!(f, "pod<{}>", name)?;
            }
            Type::Void => f.write_str("void")?,
            Type::ManagedStructure(name, _) => f.write_str(name)?,
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub source: Source,
}

impl Stmt {
    pub fn new(kind: StmtKind, source: Source) -> Self {
        Self { kind, source }
    }
}

#[derive(Clone, Debug, Unwrap)]
pub enum StmtKind {
    Return(Option<Expr>),
    Expr(TypedExpr),
    Declaration(Declaration),
    Assignment(Assignment),
}

#[derive(Clone, Debug)]
pub struct Declaration {
    pub key: VariableStorageKey,
    pub value: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct Assignment {
    pub destination: Destination,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct TypedExpr {
    pub resolved_type: Type,
    pub expr: Expr,
    pub is_initialized: bool,
}

impl TypedExpr {
    pub fn new(resolved_type: Type, expr: Expr) -> Self {
        Self {
            resolved_type,
            expr,
            is_initialized: true,
        }
    }

    pub fn new_maybe_initialized(resolved_type: Type, expr: Expr, is_initialized: bool) -> Self {
        Self {
            resolved_type,
            expr,
            is_initialized,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegerLiteralBits {
    Bits8,
    Bits16,
    Bits32,
    Bits64,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub source: Source,
}

impl Expr {
    pub fn new(kind: ExprKind, source: Source) -> Self {
        Self { kind, source }
    }
}

#[derive(Clone, Debug)]
pub enum MemoryManagement {
    None,
    ReferenceCounted,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Variable(Variable),
    GlobalVariable(GlobalVariable),
    BooleanLiteral(bool),
    IntegerLiteral(BigInt),
    Integer {
        value: BigInt,
        bits: IntegerLiteralBits,
        sign: IntegerSign,
    },
    Float(FloatSize, f64),
    String(String),
    NullTerminatedString(CString),
    Call(Call),
    DeclareAssign(DeclareAssign),
    BinaryOperation(Box<BinaryOperation>),
    IntegerExtend(Box<Expr>, Type),
    FloatExtend(Box<Expr>, Type),
    Member {
        subject: Destination,
        structure_ref: StructureRef,
        index: usize,
        memory_management: MemoryManagement,
        field_type: Type,
    },
    StructureLiteral {
        structure_type: Type,
        fields: IndexMap<String, (Expr, usize)>,
        memory_management: MemoryManagement,
    },
    UnaryOperation(Box<UnaryOperation>),
    Conditional(Conditional),
    While(While),
    ArrayAccess(Box<ArrayAccess>),
}

#[derive(Clone, Debug)]
pub struct ArrayAccess {
    pub subject: Expr,
    pub item_type: Type,
    pub index: Expr,
}

#[derive(Clone, Debug)]
pub struct Branch {
    pub condition: TypedExpr,
    pub block: Block,
}

#[derive(Clone, Debug)]
pub struct Conditional {
    pub result_type: Type,
    pub branches: Vec<Branch>,
    pub otherwise: Option<Block>,
}

#[derive(Clone, Debug)]
pub struct While {
    pub condition: Box<Expr>,
    pub block: Block,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

impl Block {
    pub fn new(stmts: Vec<Stmt>) -> Self {
        Self { stmts }
    }

    pub fn get_result_type(&self) -> Type {
        if let Some(stmt) = self.stmts.last() {
            match &stmt.kind {
                StmtKind::Return(_) => None,
                StmtKind::Expr(expr) => Some(expr.resolved_type.clone()),
                StmtKind::Declaration(_) => None,
                StmtKind::Assignment(_) => None,
            }
        } else {
            None
        }
        .unwrap_or(Type::Void)
    }
}

#[derive(Clone, Debug)]
pub struct Destination {
    pub kind: DestinationKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum DestinationKind {
    Variable(Variable),
    GlobalVariable(GlobalVariable),
    Member {
        subject: Box<Destination>,
        structure_ref: StructureRef,
        index: usize,
        field_type: Type,
        memory_management: MemoryManagement,
    },
    ArrayAccess(Box<ArrayAccess>),
}

impl TryFrom<Expr> for Destination {
    type Error = ();

    fn try_from(value: Expr) -> Result<Self, Self::Error> {
        value.kind.try_into().map(|kind| Destination {
            kind,
            source: value.source,
        })
    }
}

impl TryFrom<ExprKind> for DestinationKind {
    type Error = ();

    fn try_from(value: ExprKind) -> Result<Self, Self::Error> {
        match value {
            ExprKind::Variable(variable) => Ok(DestinationKind::Variable(variable)),
            ExprKind::GlobalVariable(global) => Ok(DestinationKind::GlobalVariable(global)),
            ExprKind::Member {
                subject,
                structure_ref,
                index,
                field_type,
                memory_management,
            } => Ok(DestinationKind::Member {
                subject: Box::new(subject),
                structure_ref,
                index,
                field_type,
                memory_management,
            }),
            ExprKind::ArrayAccess(array_access) => Ok(DestinationKind::ArrayAccess(array_access)),
            _ => Err(()),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum FloatOrSign {
    Integer(IntegerSign),
    Float,
}

#[derive(Copy, Clone, Debug)]
pub enum FloatOrInteger {
    Integer,
    Float,
}

impl From<FloatOrSign> for FloatOrInteger {
    fn from(value: FloatOrSign) -> Self {
        match value {
            FloatOrSign::Integer(_) => Self::Integer,
            FloatOrSign::Float => Self::Float,
        }
    }
}

#[derive(Clone, Debug)]
pub enum NumericMode {
    Integer(IntegerSign),
    CheckOverflow(IntegerSign),
    Float,
}

#[derive(Clone, Debug)]
pub enum BinaryOperator {
    Add(NumericMode),
    Subtract(NumericMode),
    Multiply(NumericMode),
    Divide(FloatOrSign),
    Modulus(FloatOrSign),
    Equals(FloatOrInteger),
    NotEquals(FloatOrInteger),
    LessThan(FloatOrSign),
    LessThanEq(FloatOrSign),
    GreaterThan(FloatOrSign),
    GreaterThanEq(FloatOrSign),
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
    LogicalLeftShift,
    LogicalRightShift,
}

#[derive(Clone, Debug)]
pub struct BinaryOperation {
    pub operator: BinaryOperator,
    pub left: TypedExpr,
    pub right: TypedExpr,
}

#[derive(Clone, Debug)]
pub struct UnaryOperation {
    pub operator: UnaryOperator,
    pub inner: TypedExpr,
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub key: VariableStorageKey,
    pub resolved_type: Type,
}

#[derive(Clone, Debug)]
pub struct GlobalVariable {
    pub reference: GlobalRef,
    pub resolved_type: Type,
}

#[derive(Clone, Debug)]
pub struct Call {
    pub function: FunctionRef,
    pub arguments: Vec<Expr>,
}

#[derive(Clone, Debug)]
pub struct DeclareAssign {
    pub key: VariableStorageKey,
    pub value: Box<Expr>,
    pub resolved_type: Type,
}
