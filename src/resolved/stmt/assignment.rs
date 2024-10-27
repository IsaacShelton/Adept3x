use crate::resolved::{BasicBinaryOperator, Destination, Expr};

#[derive(Clone, Debug)]
pub struct Assignment {
    pub destination: Destination,
    pub value: Expr,
    pub operator: Option<BasicBinaryOperator>,
}
