use super::Expr;
use crate::ast::Type;

#[derive(Clone, Debug)]
pub struct Call {
    pub function_name: String,
    pub arguments: Vec<Expr>,
    pub expected_to_return: Option<Type>,
}
