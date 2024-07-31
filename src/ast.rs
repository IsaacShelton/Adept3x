use crate::{
    ir::InterpreterSyscallKind,
    line_column::Location,
    resolved::IntegerLiteralBits,
    source_file_cache::{SourceFileCache, SourceFileCacheKey},
    tag::Tag,
    version::AdeptVersion,
};
use indexmap::IndexMap;
use num_bigint::BigInt;
use std::{
    cmp::Ordering,
    collections::HashMap,
    ffi::CString,
    fmt::{Debug, Display},
};

// WARNING: Don't implement PartialEq, Eq, or Hash for this.
// It's too easy to accidentally define constructs that are only equal
// depending on source, which is usually not what we want.
#[derive(Copy, Clone, Debug)]
pub struct Source {
    pub key: SourceFileCacheKey,
    pub location: Location,
}

impl Source {
    pub fn new(key: SourceFileCacheKey, location: Location) -> Self {
        Self { key, location }
    }

    pub fn internal() -> Self {
        Self {
            key: SourceFileCache::INTERNAL_KEY,
            location: Location { line: 1, column: 1 },
        }
    }

    pub fn is_internal(&self) -> bool {
        self.key == SourceFileCache::INTERNAL_KEY
    }

    pub fn shift_column(&self, amount: u32) -> Self {
        Self {
            key: self.key,
            location: self.location.shift_column(amount),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Ast<'a> {
    pub primary_filename: String,
    pub files: HashMap<FileId, AstFile>,
    pub source_file_cache: &'a SourceFileCache,
}

impl<'a> Ast<'a> {
    pub fn new(primary_filename: String, source_file_cache: &'a SourceFileCache) -> Self {
        Self {
            primary_filename,
            files: HashMap::new(),
            source_file_cache,
        }
    }

    pub fn new_file(&mut self, identifier: FileId) -> &mut AstFile {
        self.files.entry(identifier).or_insert_with(AstFile::new)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileId {
    Local(String),
    Remote {
        owner: Option<String>,
        name: String,
        version: Version,
    },
}

#[derive(Clone, Debug)]
pub struct AstModule {
    pub adept_version: AdeptVersion,
    pub functions: Vec<Function>,
    pub structures: Vec<Structure>,
    pub aliases: IndexMap<String, Alias>,
    pub globals: Vec<Global>,
    pub enums: IndexMap<String, Enum>,
    pub defines: IndexMap<String, Define>,
}

#[derive(Clone, Debug)]
pub struct AstFile {
    pub functions: Vec<Function>,
    pub structures: Vec<Structure>,
    pub aliases: IndexMap<String, Alias>,
    pub globals: Vec<Global>,
    pub enums: IndexMap<String, Enum>,
    pub defines: IndexMap<String, Define>,
}

impl AstFile {
    pub fn new() -> AstFile {
        AstFile {
            functions: vec![],
            structures: vec![],
            aliases: IndexMap::default(),
            globals: vec![],
            enums: IndexMap::default(),
            defines: IndexMap::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Global {
    pub name: String,
    pub ast_type: Type,
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
    pub ast_type: Type,
}

impl Parameter {
    pub fn new(name: String, ast_type: Type) -> Self {
        Self { name, ast_type }
    }
}

#[derive(Clone, Debug)]
pub struct Structure {
    pub name: String,
    pub fields: IndexMap<String, Field>,
    pub is_packed: bool,
    pub prefer_pod: bool,
    pub source: Source,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum Privacy {
    #[default]
    Public,
    Private,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub ast_type: Type,
    pub privacy: Privacy,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct Alias {
    pub value: Type,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct NamedAlias {
    pub name: String,
    pub alias: Alias,
}

#[derive(Clone, Debug)]
pub struct Enum {
    pub backing_type: Option<Type>,
    pub source: Source,
    pub members: IndexMap<String, EnumMember>,
}

#[derive(Clone, Debug)]
pub struct NamedEnum {
    pub name: String,
    pub enum_definition: Enum,
}

#[derive(Clone, Debug)]
pub struct Define {
    pub value: Expr,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct NamedDefine {
    pub name: String,
    pub define: Define,
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash)]
pub enum FloatSize {
    Normal,
    Bits32,
    Bits64,
}

impl FloatSize {
    pub fn bits(self) -> u8 {
        match self {
            Self::Bits32 => 32,
            Self::Bits64 | Self::Normal => 64,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IntegerBits {
    Bits8,
    Bits16,
    Bits32,
    Bits64,
    Normal,
}

impl From<IntegerLiteralBits> for IntegerBits {
    fn from(value: IntegerLiteralBits) -> Self {
        match value {
            IntegerLiteralBits::Bits8 => Self::Bits8,
            IntegerLiteralBits::Bits16 => Self::Bits16,
            IntegerLiteralBits::Bits32 => Self::Bits32,
            IntegerLiteralBits::Bits64 => Self::Bits64,
        }
    }
}

impl IntegerBits {
    pub fn new(bits: u64) -> Option<Self> {
        if bits <= 8 {
            Some(Self::Bits8)
        } else if bits <= 16 {
            Some(Self::Bits16)
        } else if bits <= 32 {
            Some(Self::Bits32)
        } else if bits <= 64 {
            Some(Self::Bits64)
        } else {
            None
        }
    }

    pub fn successor(self) -> Option<IntegerBits> {
        match self {
            Self::Normal => Some(Self::Normal),
            Self::Bits8 => Some(Self::Bits16),
            Self::Bits16 => Some(Self::Bits32),
            Self::Bits32 => Some(Self::Bits64),
            Self::Bits64 => None,
        }
    }

    pub fn bits(self) -> u8 {
        match self {
            IntegerBits::Bits8 => 8,
            IntegerBits::Bits16 => 16,
            IntegerBits::Bits32 => 32,
            IntegerBits::Bits64 | IntegerBits::Normal => 64,
        }
    }
}

impl Ord for IntegerBits {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bits().cmp(&other.bits())
    }
}

impl PartialOrd for IntegerBits {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum IntegerSign {
    Signed,
    Unsigned,
}

#[derive(Clone, Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub source: Source,
}

impl Type {
    pub fn new(kind: TypeKind, source: Source) -> Self {
        Self { kind, source }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CInteger {
    Char,
    Short,
    Int,
    Long,
    LongLong,
}

impl CInteger {
    pub fn min_bits(self) -> IntegerBits {
        match self {
            Self::Char => IntegerBits::Bits8,
            Self::Short | Self::Int => IntegerBits::Bits16,
            Self::Long => IntegerBits::Bits32,
            Self::LongLong => IntegerBits::Bits64,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TypeKind {
    Boolean,
    Integer {
        bits: IntegerBits,
        sign: IntegerSign,
    },
    CInteger {
        integer: CInteger,
        sign: Option<IntegerSign>,
    },
    Float(FloatSize),
    Pointer(Box<Type>),
    FixedArray(Box<FixedArray>),
    PlainOldData(Box<Type>),
    Void,
    Named(String),
    AnonymousStruct(AnonymousStruct),
    AnonymousUnion(AnoymousUnion),
    AnonymousEnum(AnonymousEnum),
    FunctionPointer(FunctionPointer),
}

impl TypeKind {
    pub fn at(self, source: Source) -> Type {
        Type { kind: self, source }
    }

    pub fn allow_undeclared(&self) -> bool {
        // TODO: CLEANUP: This is a bad way of doing it, should `Named` have property for this?
        // This is very rarely needed though, so it's yet to be seen if that would be an improvement.
        if let TypeKind::Named(name) = self {
            if name.starts_with("struct<") {
                return true;
            }
        }
        false
    }
}

#[derive(Clone, Debug)]
pub struct AnonymousStruct {
    pub fields: IndexMap<String, Field>,
    pub is_packed: bool,
}

#[derive(Clone, Debug)]
pub struct AnoymousUnion {}

#[derive(Clone, Debug)]
pub struct AnonymousEnum {
    pub members: IndexMap<String, EnumMember>,
    pub backing_type: Option<Box<Type>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumMember {
    pub value: BigInt,
    pub explicit_value: bool,
}

#[derive(Clone, Debug)]
pub struct FixedArray {
    pub ast_type: Type,
    pub count: Expr,
}

#[derive(Clone, Debug)]
pub struct FunctionPointer {
    pub parameters: Vec<Parameter>,
    pub return_type: Box<Type>,
    pub is_cstyle_variadic: bool,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.kind)
    }
}

impl Display for &TypeKind {
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
            TypeKind::CInteger { integer, sign } => {
                fmt_c_integer(f, *integer, *sign)?;
            }
            TypeKind::Pointer(inner) => {
                write!(f, "ptr<{inner}>")?;
            }
            TypeKind::PlainOldData(inner) => {
                write!(f, "pod<{inner}>")?;
            }
            TypeKind::Void => {
                write!(f, "void")?;
            }
            TypeKind::Named(name) => {
                write!(f, "{name}")?;
            }
            TypeKind::Float(size) => f.write_str(match size {
                FloatSize::Normal => "float",
                FloatSize::Bits32 => "f32",
                FloatSize::Bits64 => "f64",
            })?,
            TypeKind::AnonymousStruct(..) => f.write_str("(anonymous struct)")?,
            TypeKind::AnonymousUnion(..) => f.write_str("(anonymous union)")?,
            TypeKind::AnonymousEnum(..) => f.write_str("(anonymous enum)")?,
            TypeKind::FixedArray(fixed_array) => {
                write!(f, "array<(amount), {}>", fixed_array.ast_type)?;
            }
            TypeKind::FunctionPointer(_function) => {
                write!(f, "(function pointer type)")?;
            }
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

#[derive(Clone, Debug)]
pub enum StmtKind {
    Return(Option<Expr>),
    Expr(Expr),
    Declaration(Box<Declaration>),
    Assignment(Box<Assignment>),
}

impl StmtKind {
    pub fn at(self, source: Source) -> Stmt {
        Stmt { kind: self, source }
    }
}

#[derive(Clone, Debug)]
pub struct Declaration {
    pub name: String,
    pub ast_type: Type,
    pub value: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct Assignment {
    pub destination: Expr,
    pub value: Expr,
    pub operator: Option<BasicBinaryOperator>,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct ConstExpr {
    pub value: Expr,
}

impl Expr {
    pub fn new(kind: ExprKind, source: Source) -> Self {
        Self { kind, source }
    }
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Variable(String),
    Boolean(bool),
    Integer(Integer),
    Float(f64),
    String(String),
    NullTerminatedString(CString),
    Call(Box<Call>),
    DeclareAssign(Box<DeclareAssign>),
    BasicBinaryOperation(Box<BasicBinaryOperation>),
    ShortCircuitingBinaryOperation(Box<ShortCircuitingBinaryOperation>),
    Member(Box<Expr>, String),
    ArrayAccess(Box<ArrayAccess>),
    StructureLiteral(Box<StructureLiteral>),
    UnaryOperation(Box<UnaryOperation>),
    Conditional(Conditional),
    While(Box<While>),
    EnumMemberLiteral(Box<EnumMemberLiteral>),
    InterpreterSyscall(Box<InterpreterSyscall>),
}

#[derive(Clone, Debug)]
pub struct InterpreterSyscall {
    pub kind: InterpreterSyscallKind,
    pub args: Vec<(Type, Expr)>,
    pub result_type: Type,
}

#[derive(Clone, Debug)]
pub struct StructureLiteral {
    pub ast_type: Type,
    pub fields: Vec<FieldInitializer>,
    pub fill_behavior: FillBehavior,
    pub conform_behavior: ConformBehavior,
}

#[derive(Clone, Debug)]
pub struct FieldInitializer {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Copy, Clone, Debug)]
pub enum FillBehavior {
    Forbid,
    Zeroed,
}

#[derive(Copy, Clone, Debug)]
pub enum ConformBehavior {
    Adept,
    C,
}

// Make sure ExprKind doesn't accidentally become huge
const _: () = assert!(std::mem::size_of::<ExprKind>() <= 48);

impl ExprKind {
    pub fn at(self, source: Source) -> Expr {
        Expr::new(self, source)
    }
}

#[derive(Clone, Debug)]
pub enum Integer {
    Known(Box<IntegerKnown>),
    Generic(BigInt),
}

#[derive(Clone, Debug)]
pub struct IntegerKnown {
    pub bits: IntegerLiteralBits,
    pub sign: IntegerSign,
    pub value: BigInt,
}

impl Integer {
    pub fn value(&self) -> &BigInt {
        match self {
            Integer::Known(known) => &known.value,
            Integer::Generic(value) => value,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArrayAccess {
    pub subject: Expr,
    pub index: Expr,
}

#[derive(Clone, Debug)]
pub struct Conditional {
    pub conditions: Vec<(Expr, Block)>,
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
}

#[derive(Clone, Debug)]
pub enum BinaryOperator {
    Basic(BasicBinaryOperator),
    ShortCircuiting(ShortCircuitingBinaryOperator),
}

impl From<BasicBinaryOperator> for BinaryOperator {
    fn from(value: BasicBinaryOperator) -> Self {
        Self::Basic(value)
    }
}

impl From<ShortCircuitingBinaryOperator> for BinaryOperator {
    fn from(value: ShortCircuitingBinaryOperator) -> Self {
        Self::ShortCircuiting(value)
    }
}

#[derive(Clone, Debug)]
pub struct BasicBinaryOperation {
    pub operator: BasicBinaryOperator,
    pub left: Expr,
    pub right: Expr,
}

#[derive(Clone, Debug)]
pub enum BasicBinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
    Equals,
    NotEquals,
    LessThan,
    LessThanEq,
    GreaterThan,
    GreaterThanEq,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
    LogicalLeftShift,
    LogicalRightShift,
}

impl Display for BasicBinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulus => "%",
            Self::Equals => "==",
            Self::NotEquals => "!=",
            Self::LessThan => "<",
            Self::LessThanEq => "<=",
            Self::GreaterThan => ">",
            Self::GreaterThanEq => ">=",
            Self::BitwiseAnd => "&",
            Self::BitwiseOr => "|",
            Self::BitwiseXor => "^",
            Self::LeftShift => "<<",
            Self::RightShift => ">>",
            Self::LogicalLeftShift => "<<<",
            Self::LogicalRightShift => ">>>",
        })
    }
}

#[derive(Clone, Debug)]
pub struct ShortCircuitingBinaryOperation {
    pub operator: ShortCircuitingBinaryOperator,
    pub left: Expr,
    pub right: Expr,
}

#[derive(Copy, Clone, Debug)]
pub enum ShortCircuitingBinaryOperator {
    And,
    Or,
}

impl Display for ShortCircuitingBinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::And => "&&",
            Self::Or => "||",
        })
    }
}

#[derive(Clone, Debug)]
pub struct UnaryOperation {
    pub operator: UnaryOperator,
    pub inner: Expr,
}

#[derive(Clone, Debug)]
pub enum UnaryOperator {
    Not,
    BitComplement,
    Negate,
}

impl Display for UnaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Not => "!",
            Self::BitComplement => "~",
            Self::Negate => "-",
        })
    }
}

impl BasicBinaryOperator {
    pub fn returns_boolean(&self) -> bool {
        match self {
            Self::Equals
            | Self::NotEquals
            | Self::LessThan
            | Self::LessThanEq
            | Self::GreaterThan
            | Self::GreaterThanEq => true,
            Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::Modulus
            | Self::BitwiseAnd
            | Self::BitwiseOr
            | Self::BitwiseXor
            | Self::LeftShift
            | Self::RightShift
            | Self::LogicalLeftShift
            | Self::LogicalRightShift => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Call {
    pub function_name: String,
    pub arguments: Vec<Expr>,
    pub expected_to_return: Option<Type>,
}

#[derive(Clone, Debug)]
pub struct DeclareAssign {
    pub name: String,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct EnumMemberLiteral {
    pub enum_name: String,
    pub variant_name: String,
    pub source: Source,
}

pub fn fmt_c_integer(
    f: &mut std::fmt::Formatter<'_>,
    integer: CInteger,
    sign: Option<IntegerSign>,
) -> std::fmt::Result {
    match sign {
        Some(IntegerSign::Signed) => f.write_str("signed ")?,
        Some(IntegerSign::Unsigned) => f.write_str("unsigned ")?,
        None => (),
    }

    f.write_str(match integer {
        CInteger::Char => "char",
        CInteger::Short => "short",
        CInteger::Int => "int",
        CInteger::Long => "long",
        CInteger::LongLong => "long long",
    })?;

    Ok(())
}
