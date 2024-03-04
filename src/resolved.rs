use num_bigint::BigInt;
use slotmap::{new_key_type, SlotMap};
use std::{collections::HashMap, ffi::CString, fmt::Debug};

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
    pub ast_type: Type,
}

pub use crate::ast::{IntegerSign, IntegerBits};

#[derive(Clone, Debug)]
pub enum Type {
    Integer {
        bits: IntegerBits,
        sign: IntegerSign,
    },
    Pointer(Box<Type>),
    Void,
}

#[derive(Clone, Debug)]
pub enum Statement {
    Return(Option<Expression>),
    Expression(Expression),
}

#[derive(Clone, Debug)]
pub enum Expression {
    Variable(String),
    Integer(BigInt),
    NullTerminatedString(CString),
    Call(Call),
}

#[derive(Clone, Debug)]
pub struct Call {
    pub function: FunctionRef,
    pub arguments: Vec<Expression>,
}

