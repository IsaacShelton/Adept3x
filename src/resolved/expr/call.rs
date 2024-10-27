use super::TypedExpr;
use crate::resolved::FunctionRef;

#[derive(Clone, Debug)]
pub struct Call {
    pub function: FunctionRef,
    pub arguments: Vec<TypedExpr>,
}
