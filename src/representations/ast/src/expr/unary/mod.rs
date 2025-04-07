mod math;

use crate::Expr;
pub use math::*;

#[derive(Clone, Debug)]
pub enum UnaryOperator {
    Math(UnaryMathOperator),
    AddressOf,
    Dereference,
}

#[derive(Clone, Debug)]
pub struct UnaryOperation {
    pub operator: UnaryOperator,
    pub inner: Expr,
}

impl UnaryOperation {
    pub fn new(operator: UnaryOperator, inner: Expr) -> Self {
        Self { operator, inner }
    }

    pub fn new_math(operator: UnaryMathOperator, inner: Expr) -> Self {
        Self {
            operator: UnaryOperator::Math(operator),
            inner,
        }
    }
}
