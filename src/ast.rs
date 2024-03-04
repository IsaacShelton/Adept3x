use std::collections::HashMap;

use num_bigint::BigInt;

#[derive(Clone, Debug)]
pub struct Ast {
    pub primary_filename: String,
    pub files: HashMap<FileIdentifier, File>,
}

impl Ast {
    pub fn new(primary_filename: String) -> Self {
        Self {
            primary_filename,
            files: HashMap::new(),
        }
    }

    pub fn new_file(&mut self, identifier: FileIdentifier) -> &mut File {
        self.files.entry(identifier).or_insert_with(|| File::new())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Version {
    major: i32,
    minor: i32,
    patch: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileIdentifier {
    Local(String),
    Remote {
        owner: Option<String>,
        name: String,
        version: Version,
    }
}

#[derive(Clone, Debug)]
pub struct File {
    pub functions: Vec<Function>,
}

impl File {
    pub fn new() -> File {
        File {
            functions: vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub statements: Vec<Statement>,
    pub is_foreign: bool,
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: String,
    pub ast_type: Type,
}

#[derive(Clone, Debug)]
pub enum IntegerBits {
    Normal,
    Bits8,
    Bits16,
    Bits32,
    Bits64,
}

#[derive(Clone, Debug)]
pub enum IntegerSign {
    Signed,
    Unsigned,
}

#[derive(Clone, Debug)]
pub enum Type {
    Integer { bits: IntegerBits, sign: IntegerSign },
    Pointer(Box<Type>),
    Void,
}

#[derive(Clone, Debug)]
pub enum Statement {
    Return(Option<Expression>),
}

#[derive(Clone, Debug)]
pub enum Expression {
    Integer(BigInt),
}
