use crate::ast::Expr;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct ShortCircuitingBinaryOperation {
    pub operator: ShortCircuitingBinaryOperator,
    pub left: Expr,
    pub right: Expr,
}

#[derive(Copy, Clone, Debug)]
pub enum ShortCircuitingBinaryOperator {
    And,
    Or,
}

impl Display for ShortCircuitingBinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::And => "&&",
            Self::Or => "||",
        })
    }
}
