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
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: String,
    pub ast_type: Type,
}

#[derive(Clone, Debug)]
pub enum Type {
    Integer,
}

#[derive(Clone, Debug)]
pub enum Statement {
    Return(Expression),
}

#[derive(Clone, Debug)]
pub enum Expression {
    Integer(BigInt),
}
