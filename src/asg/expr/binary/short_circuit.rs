pub use crate::ast::ShortCircuitingBinaryOperator;
use crate::asg::TypedExpr;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ShortCircuitingBinaryOperation {
    pub operator: ShortCircuitingBinaryOperator,
    pub left: TypedExpr,
    pub right: TypedExpr,
}
