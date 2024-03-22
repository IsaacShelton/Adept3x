use indexmap::IndexMap;
use num_traits::Zero;
mod variable_storage;

use crate::{ast::Source, source_file_cache::SourceFileCache};
use num_bigint::BigInt;
use slotmap::{new_key_type, SlotMap};
use std::{
    ffi::CString,
    fmt::{Debug, Display},
};

pub use crate::ast::{BinaryOperator, UnaryOperator};
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
    pub statements: Vec<Statement>,
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

pub use self::variable_storage::VariableStorageKey;
pub use crate::ast::{IntegerBits, IntegerSign};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Boolean,
    Integer {
        bits: IntegerBits,
        sign: IntegerSign,
    },
    IntegerLiteral(BigInt),
    Pointer(Box<Type>),
    PlainOldData(String, StructureRef),
    Void,
    Structure(String, StructureRef),
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
            Type::Pointer(_) => None,
            Type::PlainOldData(_, _) => None,
            Type::Void => None,
            Type::Structure(_, _) => None,
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
            Type::Pointer(inner) => {
                write!(f, "ptr<{}>", inner)?;
            }
            Type::PlainOldData(name, _) => {
                write!(f, "pod<{}>", name)?;
            }
            Type::Void => f.write_str("void")?,
            Type::Structure(name, _) => f.write_str(name)?,
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Statement {
    pub kind: StatementKind,
    pub source: Source,
}

impl Statement {
    pub fn new(kind: StatementKind, source: Source) -> Self {
        Self { kind, source }
    }
}

#[derive(Clone, Debug)]
pub enum StatementKind {
    Return(Option<Expression>),
    Expression(Expression),
    Declaration(Declaration),
    Assignment(Assignment),
}

#[derive(Clone, Debug)]
pub struct Declaration {
    pub key: VariableStorageKey,
    pub value: Option<Expression>,
}

#[derive(Clone, Debug)]
pub struct Assignment {
    pub destination: Destination,
    pub value: Expression,
}

#[derive(Clone, Debug)]
pub struct TypedExpression {
    pub resolved_type: Type,
    pub expression: Expression,
    pub is_initialized: bool,
}

impl TypedExpression {
    pub fn new(resolved_type: Type, expression: Expression) -> Self {
        Self {
            resolved_type,
            expression,
            is_initialized: true,
        }
    }

    pub fn new_maybe_initialized(
        resolved_type: Type,
        expression: Expression,
        is_initialized: bool,
    ) -> Self {
        Self {
            resolved_type,
            expression,
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
pub struct Expression {
    pub kind: ExpressionKind,
    pub source: Source,
}

impl Expression {
    pub fn new(kind: ExpressionKind, source: Source) -> Self {
        Self { kind, source }
    }
}

#[derive(Clone, Debug)]
pub enum ExpressionKind {
    Variable(Variable),
    GlobalVariable(GlobalVariable),
    IntegerLiteral(BigInt),
    Integer {
        value: BigInt,
        bits: IntegerLiteralBits,
        sign: IntegerSign,
    },
    NullTerminatedString(CString),
    Call(Call),
    DeclareAssign(DeclareAssign),
    BinaryOperation(Box<BinaryOperation>),
    IntegerExtend(Box<Expression>, Type),
    Member(Destination, StructureRef, usize, Type),
    StructureLiteral(Type, IndexMap<String, (Expression, usize)>),
    UnaryOperator(Box<UnaryOperation>),
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
    Member(Box<Destination>, StructureRef, usize, Type),
}

impl TryFrom<Expression> for Destination {
    type Error = ();

    fn try_from(value: Expression) -> Result<Self, Self::Error> {
        value.kind.try_into().map(|kind| Destination {
            kind,
            source: value.source,
        })
    }
}

impl TryFrom<ExpressionKind> for DestinationKind {
    type Error = ();

    fn try_from(value: ExpressionKind) -> Result<Self, Self::Error> {
        match value {
            ExpressionKind::Variable(variable) => Ok(DestinationKind::Variable(variable)),
            ExpressionKind::GlobalVariable(global) => Ok(DestinationKind::GlobalVariable(global)),
            ExpressionKind::Member(destination, structure_ref, index, ir_type) => Ok(
                DestinationKind::Member(Box::new(destination), structure_ref, index, ir_type),
            ),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BinaryOperation {
    pub operator: BinaryOperator,
    pub left: TypedExpression,
    pub right: TypedExpression,
}

#[derive(Clone, Debug)]
pub struct UnaryOperation {
    pub operator: UnaryOperator,
    pub inner: TypedExpression,
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
    pub arguments: Vec<Expression>,
}

#[derive(Clone, Debug)]
pub struct DeclareAssign {
    pub key: VariableStorageKey,
    pub value: Box<Expression>,
    pub resolved_type: Type,
}
