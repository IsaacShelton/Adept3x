pub use crate::ast::ShortCircuitingBinaryOperator;
use crate::resolved::TypedExpr;

#[derive(Clone, Debug)]
pub struct ShortCircuitingBinaryOperation {
    pub operator: ShortCircuitingBinaryOperator,
    pub left: TypedExpr,
    pub right: TypedExpr,
}
