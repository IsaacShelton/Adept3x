mod variable_storage;

use crate::{ast::Source, source_file_cache::SourceFileCache};
use num_bigint::BigInt;
use slotmap::{new_key_type, SlotMap};
use std::{
    collections::HashMap,
    ffi::CString,
    fmt::{Debug, Display},
};

pub use variable_storage::VariableStorage;
pub use crate::ast::BinaryOperator;

new_key_type! {
    pub struct FunctionRef;
    pub struct GlobalRef;
}

#[derive(Clone, Debug)]
pub struct Ast<'a> {
    pub source_file_cache: &'a SourceFileCache,
    pub entry_point: Option<FunctionRef>,
    pub functions: SlotMap<FunctionRef, Function>,
    pub globals: SlotMap<GlobalRef, Global>,
}

impl<'a> Ast<'a> {
    pub fn new(source_file_cache: &'a SourceFileCache) -> Self {
        Self {
            source_file_cache,
            entry_point: None,
            functions: SlotMap::with_key(),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Boolean,
    Integer {
        bits: IntegerBits,
        sign: IntegerSign,
    },
    IntegerLiteral(BigInt),
    Pointer(Box<Type>),
    Void,
}

impl Type {
    pub fn sign(&self) -> Option<IntegerSign> {
        match self {
            Type::Boolean => None,
            Type::Integer { bits, sign } => Some(sign.clone()),
            Type::IntegerLiteral(value) => Some(IntegerSign::Unsigned),
            Type::Pointer(_) => None,
            Type::Void => None,
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
                write!(f, "literal integer {}", value)?;
            }
            Type::Pointer(inner) => {
                write!(f, "ptr<{}>", inner)?;
            }
            Type::Void => f.write_str("void")?,
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
}

#[derive(Clone, Debug)]
pub struct TypedExpression {
    pub resolved_type: Type,
    pub expression: Expression,
}

impl TypedExpression {
    pub fn new(resolved_type: Type, expression: Expression) -> Self {
        Self {
            resolved_type,
            expression,
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
}

#[derive(Clone, Debug)]
pub struct BinaryOperation {
    pub operator: BinaryOperator,
    pub left: TypedExpression,
    pub right: TypedExpression,
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
