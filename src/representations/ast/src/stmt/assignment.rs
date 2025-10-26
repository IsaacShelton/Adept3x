use crate::{BasicBinaryOperator, ConformBehavior, Expr};

#[derive(Clone, Debug)]
pub struct Assignment {
    pub destination: Expr,
    pub value: Expr,
    pub operator: Option<BasicBinaryOperator>,
    pub conform_behavior: ConformBehavior,
}
