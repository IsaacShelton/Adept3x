use crate::TypedExpr;
pub use ast::ShortCircuitingBinaryOperator;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ShortCircuitingBinaryOperation {
    pub operator: ShortCircuitingBinaryOperator,
    pub left: TypedExpr,
    pub right: TypedExpr,
}
