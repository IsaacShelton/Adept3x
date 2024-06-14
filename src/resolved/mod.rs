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
use thin_vec::ThinVec;

pub use self::variable_storage::VariableStorageKey;
pub use crate::ast::EnumMember;
pub use crate::ast::EnumMemberLiteral;
pub use crate::ast::ShortCircuitingBinaryOperator;
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
    pub enums: IndexMap<String, Enum>,
}

impl<'a> Ast<'a> {
    pub fn new(source_file_cache: &'a SourceFileCache) -> Self {
        Self {
            source_file_cache,
            entry_point: None,
            functions: SlotMap::with_key(),
            structures: SlotMap::with_key(),
            globals: SlotMap::with_key(),
            enums: IndexMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Enum {
    pub name: String,
    pub resolved_type: Type,
    pub source: Source,
    pub members: IndexMap<String, EnumMember>,
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
    pub source: Source,
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

impl PartialEq for Parameter {
    fn eq(&self, other: &Self) -> bool {
        self.resolved_type.eq(&other.resolved_type)
    }
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

#[derive(Clone, Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub source: Source,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.kind, f)
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.kind.eq(&other.kind)
    }
}

#[derive(Clone, Debug, PartialEq, IsVariant)]
pub enum TypeKind {
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
    AnonymousStruct(),
    AnonymousUnion(),
    FixedArray(Box<FixedArray>),
    FunctionPointer(FunctionPointer),
    Enum(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixedArray {
    pub size: u64,
    pub inner: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionPointer {
    pub parameters: Vec<Parameter>,
    pub return_type: Box<Type>,
    pub is_cstyle_variadic: bool,
}

impl TypeKind {
    pub fn at(self, source: Source) -> Type {
        Type { kind: self, source }
    }

    pub fn sign(&self) -> Option<IntegerSign> {
        match self {
            TypeKind::Boolean => None,
            TypeKind::Integer { sign, .. } => Some(*sign),
            TypeKind::IntegerLiteral(value) => Some(if value >= &BigInt::zero() {
                IntegerSign::Unsigned
            } else {
                IntegerSign::Signed
            }),
            TypeKind::Float(_) => None,
            TypeKind::FloatLiteral(_) => None,
            TypeKind::Pointer(_) => None,
            TypeKind::PlainOldData(_, _) => None,
            TypeKind::Void => None,
            TypeKind::ManagedStructure(_, _) => None,
            TypeKind::AnonymousStruct(..) => None,
            TypeKind::AnonymousUnion(..) => None,
            TypeKind::FixedArray(..) => None,
            TypeKind::FunctionPointer(..) => None,
            TypeKind::Enum(_) => None,
        }
    }

    pub fn is_void_pointer(&self) -> bool {
        match self {
            TypeKind::Pointer(inner) if inner.kind.is_void() => true,
            _ => false,
        }
    }
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeKind::Boolean => {
                write!(f, "bool")?;
            }
            TypeKind::Integer { bits, sign } => {
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
            TypeKind::IntegerLiteral(value) => {
                write!(f, "integer {}", value)?;
            }
            TypeKind::Float(size) => match size {
                FloatSize::Normal => f.write_str("float")?,
                FloatSize::Bits32 => f.write_str("f32")?,
                FloatSize::Bits64 => f.write_str("f64")?,
            },
            TypeKind::FloatLiteral(value) => write!(f, "float {}", value)?,
            TypeKind::Pointer(inner) => {
                write!(f, "ptr<{}>", inner.kind)?;
            }
            TypeKind::PlainOldData(name, _) => {
                write!(f, "pod<{}>", name)?;
            }
            TypeKind::Void => f.write_str("void")?,
            TypeKind::ManagedStructure(name, _) => f.write_str(name)?,
            TypeKind::AnonymousStruct() => f.write_str("(anonymous struct)")?,
            TypeKind::AnonymousUnion() => f.write_str("(anonymous union)")?,
            TypeKind::FixedArray(fixed_array) => {
                write!(f, "array<{}, {}>", fixed_array.size, fixed_array.inner.kind)?;
            }
            TypeKind::FunctionPointer(..) => f.write_str("(function pointer type)")?,
            TypeKind::Enum(enum_name) => write!(f, "(enum) {}", enum_name)?,
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub source: Source,
    pub drops: Drops,
}

impl Stmt {
    pub fn new(kind: StmtKind, source: Source) -> Self {
        Self {
            kind,
            source,
            drops: Drops::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Drops {
    pub drops: ThinVec<VariableStorageKey>,
}

impl Drops {
    pub fn push(&mut self, variable: VariableStorageKey) {
        self.drops.push(variable);
    }

    pub fn iter(&self) -> impl Iterator<Item = &VariableStorageKey> + '_ {
        self.drops.iter()
    }
}

#[derive(Clone, Debug, Unwrap)]
pub enum StmtKind {
    Return(Option<Expr>, Drops),
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
    pub operator: Option<BasicBinaryOperator>,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
    BasicBinaryOperation(Box<BasicBinaryOperation>),
    ShortCircuitingBinaryOperation(Box<ShortCircuitingBinaryOperation>),
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
    EnumMemberLiteral(EnumMemberLiteral),
}

impl ExprKind {
    pub fn at(self, source: Source) -> Expr {
        Expr::new(self, source)
    }
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

    pub fn get_result_type(&self, source: Source) -> Type {
        if let Some(stmt) = self.stmts.last() {
            match &stmt.kind {
                StmtKind::Return(..) => None,
                StmtKind::Expr(expr) => Some(expr.resolved_type.clone()),
                StmtKind::Declaration(..) => None,
                StmtKind::Assignment(..) => None,
            }
        } else {
            None
        }
        .unwrap_or(TypeKind::Void.at(source))
    }
}

#[derive(Clone, Debug)]
pub struct Destination {
    pub kind: DestinationKind,
    pub resolved_type: Type,
    pub source: Source,
}

impl Destination {
    pub fn new(kind: DestinationKind, resolved_type: Type, source: Source) -> Self {
        Self {
            kind,
            source,
            resolved_type,
        }
    }
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
pub enum BasicBinaryOperator {
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
pub struct BasicBinaryOperation {
    pub operator: BasicBinaryOperator,
    pub left: TypedExpr,
    pub right: TypedExpr,
}

#[derive(Clone, Debug)]
pub struct ShortCircuitingBinaryOperation {
    pub operator: ShortCircuitingBinaryOperator,
    pub left: TypedExpr,
    pub right: TypedExpr,
    pub drops: Drops,
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
