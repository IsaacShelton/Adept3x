use super::TypedExpr;
pub use crate::ast::UnaryMathOperator;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct UnaryMathOperation {
    pub operator: UnaryMathOperator,
    pub inner: TypedExpr,
}
