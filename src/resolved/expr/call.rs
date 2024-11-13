use super::TypedExpr;
use crate::resolved::{FunctionRef, PolyValue};
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct Call {
    pub callee: Callee,
    pub arguments: Vec<TypedExpr>,
}

#[derive(Clone, Debug)]
pub struct Callee {
    pub function: FunctionRef,
    pub recipe: IndexMap<String, PolyValue>,
}
