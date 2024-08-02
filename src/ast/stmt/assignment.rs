use crate::ast::{BasicBinaryOperator, Expr};

#[derive(Clone, Debug)]
pub struct Assignment {
    pub destination: Expr,
    pub value: Expr,
    pub operator: Option<BasicBinaryOperator>,
}
