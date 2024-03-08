mod variable_storage;

use num_bigint::BigInt;
use slotmap::{new_key_type, SlotMap};
use std::{
    collections::HashMap,
    ffi::CString,
    fmt::{Debug, Display},
};
pub use variable_storage::VariableStorage;

new_key_type! {
    pub struct FunctionRef;
}

#[derive(Clone, Debug)]
pub struct Ast {
    pub entry_point: Option<FunctionRef>,
    pub functions: SlotMap<FunctionRef, Function>,
}

impl Default for Ast {
    fn default() -> Self {
        Self {
            entry_point: None,
            functions: SlotMap::with_key(),
        }
    }
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
    Integer {
        bits: IntegerBits,
        sign: IntegerSign,
    },
    IntegerLiteral(BigInt),
    Pointer(Box<Type>),
    Void,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Integer { bits, sign } => {
                f.write_str(match (bits, sign) {
                    (IntegerBits::Normal, IntegerSign::Signed) => "int",
                    (IntegerBits::Normal, IntegerSign::Unsigned) => "uint",
                    (IntegerBits::Bits8, IntegerSign::Signed) => "int8",
                    (IntegerBits::Bits8, IntegerSign::Unsigned) => "uint8",
                    (IntegerBits::Bits16, IntegerSign::Signed) => "int16",
                    (IntegerBits::Bits16, IntegerSign::Unsigned) => "uint16",
                    (IntegerBits::Bits32, IntegerSign::Signed) => "int32",
                    (IntegerBits::Bits32, IntegerSign::Unsigned) => "uint32",
                    (IntegerBits::Bits64, IntegerSign::Signed) => "int64",
                    (IntegerBits::Bits64, IntegerSign::Unsigned) => "uint64",
                })?;
            }
            Type::IntegerLiteral(value) => {
                write!(f, "literal integer {}", value)?;
            }
            Type::Pointer(inner) => {
                f.write_str("ptr:")?;
                write!(f, "{}", inner)?;
            }
            Type::Void => f.write_str("void")?,
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Statement {
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
pub enum Expression {
    Variable(Variable),
    IntegerLiteral(BigInt),
    Integer {
        value: BigInt,
        bits: IntegerLiteralBits,
        sign: IntegerSign,
    },
    NullTerminatedString(CString),
    Call(Call),
    DeclareAssign(DeclareAssign),
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub key: VariableStorageKey,
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
}
