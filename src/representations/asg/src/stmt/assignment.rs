use crate::{BasicBinaryOperator, Destination, Expr};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Assignment {
    pub destination: Destination,
    pub value: Expr,
    pub operator: Option<BasicBinaryOperator>,
}
