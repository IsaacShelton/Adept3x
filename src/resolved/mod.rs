mod variable_storage;

pub use self::variable_storage::VariableStorageKey;
pub use crate::ast::{
    CInteger, EnumMember, EnumMemberLiteral, FloatSize, IntegerBits, IntegerKnown, IntegerSign,
    ShortCircuitingBinaryOperator, UnaryOperator,
};
use crate::{
    ast::fmt_c_integer,
    ir::InterpreterSyscallKind,
    source_files::{Source, SourceFiles},
    tag::Tag,
    target::Target,
};
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
pub use variable_storage::VariableStorage;

new_key_type! {
    pub struct FunctionRef;
    pub struct GlobalVarRef;
    pub struct StructureRef;
}

#[derive(Clone, Debug)]
pub struct Ast<'a> {
    pub source_files: &'a SourceFiles,
    pub entry_point: Option<FunctionRef>,
    pub functions: SlotMap<FunctionRef, Function>,
    pub structures: SlotMap<StructureRef, Structure>,
    pub globals: SlotMap<GlobalVarRef, GlobalVar>,
    pub enums: IndexMap<String, Enum>,
}

impl<'a> Ast<'a> {
    pub fn new(source_files: &'a SourceFiles) -> Self {
        Self {
            source_files,
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
    pub resolved_type: Type,
    pub source: Source,
    pub members: IndexMap<String, EnumMember>,
}

#[derive(Clone, Debug)]
pub struct GlobalVar {
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
    pub abide_abi: bool,
    pub tag: Option<Tag>,
}

#[derive(Clone, Debug, Default)]
pub struct Parameters {
    pub required: Vec<Parameter>,
    pub is_cstyle_vararg: bool,
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
    pub source: Source,
}

pub use crate::ast::Privacy;

#[derive(Clone, Debug)]
pub struct Field {
    pub resolved_type: Type,
    pub privacy: Privacy,
    pub source: Source,
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

#[derive(Clone, Debug, PartialEq, IsVariant, Unwrap)]
pub enum TypeKind {
    Boolean,
    Integer(IntegerBits, IntegerSign),
    CInteger(CInteger, Option<IntegerSign>),
    IntegerLiteral(BigInt),
    FloatLiteral(f64),
    Floating(FloatSize),
    Pointer(Box<Type>),
    PlainOldData(String, StructureRef),
    Void,
    ManagedStructure(String, StructureRef),
    AnonymousStruct(),
    AnonymousUnion(),
    AnonymousEnum(AnonymousEnum),
    FixedArray(Box<FixedArray>),
    FunctionPointer(FunctionPointer),
    Enum(String),
}

impl TypeKind {
    pub fn i8() -> Self {
        Self::Integer(IntegerBits::Bits8, IntegerSign::Signed)
    }

    pub fn u8() -> Self {
        Self::Integer(IntegerBits::Bits8, IntegerSign::Unsigned)
    }

    pub fn i16() -> Self {
        Self::Integer(IntegerBits::Bits16, IntegerSign::Signed)
    }

    pub fn u16() -> Self {
        Self::Integer(IntegerBits::Bits16, IntegerSign::Unsigned)
    }

    pub fn i32() -> Self {
        Self::Integer(IntegerBits::Bits32, IntegerSign::Signed)
    }

    pub fn u32() -> Self {
        Self::Integer(IntegerBits::Bits32, IntegerSign::Unsigned)
    }

    pub fn i64() -> Self {
        Self::Integer(IntegerBits::Bits64, IntegerSign::Signed)
    }

    pub fn u64() -> Self {
        Self::Integer(IntegerBits::Bits64, IntegerSign::Unsigned)
    }

    pub fn f32() -> Self {
        Self::Floating(FloatSize::Bits32)
    }

    pub fn f64() -> Self {
        Self::Floating(FloatSize::Bits64)
    }

    pub fn signed(bits: IntegerBits) -> Self {
        Self::Integer(bits, IntegerSign::Signed)
    }
    pub fn unsigned(bits: IntegerBits) -> Self {
        Self::Integer(bits, IntegerSign::Unsigned)
    }

    pub fn is_integer_like(&self) -> bool {
        matches!(
            self,
            Self::Integer(..) | Self::IntegerLiteral(..) | Self::CInteger(..)
        )
    }
}

#[derive(Clone, Debug)]
pub struct AnonymousEnum {
    pub resolved_type: Box<Type>,
    pub source: Source,
    pub members: IndexMap<String, EnumMember>,
}

impl PartialEq for AnonymousEnum {
    fn eq(&self, other: &Self) -> bool {
        self.resolved_type.kind.eq(&other.resolved_type.kind) && self.members.eq(&other.members)
    }
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

    pub fn sign(&self, target: Option<&Target>) -> Option<IntegerSign> {
        match self {
            TypeKind::Boolean => None,
            TypeKind::Integer(_, sign) => Some(*sign),
            TypeKind::IntegerLiteral(value) => Some(if value >= &BigInt::zero() {
                IntegerSign::Unsigned
            } else {
                IntegerSign::Signed
            }),
            TypeKind::CInteger(integer, sign) => {
                if let Some(sign) = sign {
                    Some(*sign)
                } else {
                    target.map(|target| target.default_c_integer_sign(*integer))
                }
            }
            TypeKind::Floating(_)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Pointer(_)
            | TypeKind::PlainOldData(_, _)
            | TypeKind::Void
            | TypeKind::ManagedStructure(_, _)
            | TypeKind::AnonymousStruct(..)
            | TypeKind::AnonymousUnion(..)
            | TypeKind::FixedArray(..)
            | TypeKind::FunctionPointer(..)
            | TypeKind::Enum(_)
            | TypeKind::AnonymousEnum(_) => None,
        }
    }

    pub fn is_void_pointer(&self) -> bool {
        matches!(self, TypeKind::Pointer(inner) if inner.kind.is_void())
    }
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeKind::Boolean => {
                write!(f, "bool")?;
            }
            TypeKind::Integer(bits, sign) => {
                f.write_str(match (bits, sign) {
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
            TypeKind::CInteger(integer, sign) => {
                fmt_c_integer(f, *integer, *sign)?;
            }
            TypeKind::IntegerLiteral(value) => {
                write!(f, "integer {}", value)?;
            }
            TypeKind::Floating(size) => match size {
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
            TypeKind::AnonymousEnum(..) => f.write_str("(anonymous enum)")?,
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
    Variable(Box<Variable>),
    GlobalVariable(Box<GlobalVariable>),
    BooleanLiteral(bool),
    IntegerLiteral(BigInt),
    IntegerKnown(Box<IntegerKnown>),
    FloatingLiteral(FloatSize, f64),
    String(String),
    NullTerminatedString(CString),
    Call(Box<Call>),
    DeclareAssign(Box<DeclareAssign>),
    BasicBinaryOperation(Box<BasicBinaryOperation>),
    ShortCircuitingBinaryOperation(Box<ShortCircuitingBinaryOperation>),
    IntegerCast(Box<CastFrom>),
    IntegerExtend(Box<Cast>),
    IntegerTruncate(Box<Cast>),
    FloatExtend(Box<Cast>),
    Member(Box<Member>),
    StructureLiteral(Box<StructureLiteral>),
    UnaryOperation(Box<UnaryOperation>),
    Conditional(Box<Conditional>),
    While(Box<While>),
    ArrayAccess(Box<ArrayAccess>),
    EnumMemberLiteral(Box<EnumMemberLiteral>),
    ResolvedNamedExpression(String, Box<Expr>),
    Zeroed(Box<Type>),
    InterpreterSyscall(InterpreterSyscallKind, Vec<Expr>),
}

#[derive(Clone, Debug)]
pub struct CastFrom {
    pub cast: Cast,
    pub from_type: Type,
}

#[derive(Clone, Debug)]
pub struct Cast {
    pub target_type: Type,
    pub value: Expr,
}

impl Cast {
    pub fn new(target_type: Type, value: Expr) -> Self {
        Self { target_type, value }
    }
}

#[derive(Clone, Debug)]
pub struct Member {
    pub subject: Destination,
    pub structure_ref: StructureRef,
    pub index: usize,
    pub memory_management: MemoryManagement,
    pub field_type: Type,
}

#[derive(Clone, Debug)]
pub struct StructureLiteral {
    pub structure_type: Type,
    pub fields: IndexMap<String, (Expr, usize)>,
    pub memory_management: MemoryManagement,
}

// Make sure ExprKind doesn't accidentally become huge
const _: () = assert!(std::mem::size_of::<ExprKind>() <= 40);

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
    pub condition: Expr,
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
pub enum FloatOrSignLax {
    Integer(IntegerSign),
    IndeterminateInteger(CInteger),
    Float,
}

impl FloatOrSignLax {
    pub fn or_default_for(&self, target: &Target) -> FloatOrSign {
        match self {
            FloatOrSignLax::Integer(sign) => FloatOrSign::Integer(*sign),
            FloatOrSignLax::IndeterminateInteger(c_integer) => {
                FloatOrSign::Integer(target.default_c_integer_sign(*c_integer))
            }
            FloatOrSignLax::Float => FloatOrSign::Float,
        }
    }
}

impl From<FloatOrSignLax> for FloatOrInteger {
    fn from(value: FloatOrSignLax) -> Self {
        match value {
            FloatOrSignLax::Integer(_) | FloatOrSignLax::IndeterminateInteger(_) => Self::Integer,
            FloatOrSignLax::Float => Self::Float,
        }
    }
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
    LooseIndeterminateSignInteger(CInteger),
    CheckOverflow(IntegerBits, IntegerSign),
    Float,
}

#[derive(Clone, Debug)]
pub enum SignOrIndeterminate {
    Sign(IntegerSign),
    Indeterminate(CInteger),
}

#[derive(Clone, Debug)]
pub enum BasicBinaryOperator {
    Add(NumericMode),
    Subtract(NumericMode),
    Multiply(NumericMode),
    Divide(FloatOrSignLax),
    Modulus(FloatOrSignLax),
    Equals(FloatOrInteger),
    NotEquals(FloatOrInteger),
    LessThan(FloatOrSignLax),
    LessThanEq(FloatOrSignLax),
    GreaterThan(FloatOrSignLax),
    GreaterThanEq(FloatOrSignLax),
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    ArithmeticRightShift(SignOrIndeterminate),
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
    pub reference: GlobalVarRef,
    pub resolved_type: Type,
}

#[derive(Clone, Debug)]
pub struct Call {
    pub function: FunctionRef,
    pub arguments: Vec<TypedExpr>,
}

#[derive(Clone, Debug)]
pub struct DeclareAssign {
    pub key: VariableStorageKey,
    pub value: Expr,
    pub resolved_type: Type,
}
