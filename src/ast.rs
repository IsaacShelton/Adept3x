use num_bigint::BigInt;

#[derive(Clone, Debug)]
pub struct Ast {
    pub functions: Vec<Function>,
}

impl Ast {
    pub fn new() -> Ast {
        Ast {
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
