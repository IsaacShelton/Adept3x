use super::TypedExpr;
pub use crate::ast::UnaryMathOperator;

#[derive(Clone, Debug)]
pub struct UnaryMathOperation {
    pub operator: UnaryMathOperator,
    pub inner: TypedExpr,
}
