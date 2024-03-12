use num_bigint::BigInt;
use std::{collections::HashMap, ffi::CString, fmt::Debug};

use crate::{
    line_column::Location,
    source_file_cache::{self, SourceFileCache, SourceFileCacheKey},
};

#[derive(Copy, Clone, Debug)]
pub struct Source {
    pub key: SourceFileCacheKey,
    pub location: Location,
}

impl Source {
    pub fn new(key: SourceFileCacheKey, location: Location) -> Self {
        Self { key, location }
    }
}

#[derive(Clone, Debug)]
pub struct Ast<'a> {
    pub primary_filename: String,
    pub files: HashMap<FileIdentifier, File>,
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

    pub fn new_file(&mut self, identifier: FileIdentifier) -> &mut File {
        self.files.entry(identifier).or_insert_with(|| File::new())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileIdentifier {
    Local(String),
    Remote {
        owner: Option<String>,
        name: String,
        version: Version,
    },
}

#[derive(Clone, Debug)]
pub struct File {
    pub functions: Vec<Function>,
    pub globals: Vec<Global>,
}

impl File {
    pub fn new() -> File {
        File {
            functions: vec![],
            globals: vec![],
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegerBits {
    Normal,
    Bits8,
    Bits16,
    Bits32,
    Bits64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegerSign {
    Signed,
    Unsigned,
}

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
    Variable(String),
    Integer(BigInt),
    NullTerminatedString(CString),
    Call(Call),
    DeclareAssign(DeclareAssign),
    BinaryOperation(Box<BinaryOperation>),
}

#[derive(Clone, Debug)]
pub struct BinaryOperation {
    pub operator: BinaryOperator,
    pub left: Expression,
    pub right: Expression,
}

#[derive(Clone, Debug)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
}

#[derive(Clone, Debug)]
pub struct Call {
    pub function_name: String,
    pub arguments: Vec<Expression>,
}

#[derive(Clone, Debug)]
pub struct DeclareAssign {
    pub name: String,
    pub value: Box<Expression>,
}
