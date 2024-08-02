use crate::ast::{Expr, Type};

#[derive(Clone, Debug)]
pub struct Declaration {
    pub name: String,
    pub ast_type: Type,
    pub value: Option<Expr>,
}
